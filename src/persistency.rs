use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use anyhow::{Context, Result};

use crate::nobject::{IriIndex, NodeCache, StringIndexer};
use crate::RdfGlanceApp;

// it is just ascii "rdfg"
const MAGIC_NUMBER: u32 = 0x47464452;
const FORMAT_VERSION: u16 = 0;
const FORMAT_FLAGS: u16 = 0;

#[repr(u8)]
pub enum HeaderType {
    Predicates = 0x01,
    Types = 0x02,
    Languages = 0x03,
    DataTypes = 0x04,
    Nodes = 0x05,
}

impl HeaderType {
    pub fn from_u8(value: u8) -> Option<HeaderType> {
        match value {
            1 => Some(HeaderType::Predicates),
            2 => Some(HeaderType::Types),
            3 => Some(HeaderType::Languages),
            4 => Some(HeaderType::DataTypes),
            5 => Some(HeaderType::Nodes),
            _ => None,
        }
    }
}

impl RdfGlanceApp {
    
    pub fn store(&self, path: &Path)  -> std::io::Result<()> {
        let mut file = BufWriter::new(File::create(path)?);

        file.write_u32::<LittleEndian>(MAGIC_NUMBER)?;
        file.write_u16::<LittleEndian>(FORMAT_VERSION)?;
        file.write_u16::<LittleEndian>(FORMAT_FLAGS)?;
        // header size 
        file.write_u16::<LittleEndian>(10)?;

        self.node_data.indexers.predicate_indexer.store(HeaderType::Predicates as u8, &mut file)?;
        self.node_data.indexers.type_indexer.store(HeaderType::Types as u8, &mut file)?;
        self.node_data.indexers.language_indexer.store(HeaderType::Languages as u8, &mut file)?;
        self.node_data.indexers.datatype_indexer.store(HeaderType::DataTypes as u8, &mut file)?;


        file.flush()?;
        Ok(())
    }

    pub fn restore(path: &Path) -> Result<Self> {
        let mut app = RdfGlanceApp::new(None);
        let mut file = File::open(path)?;
        let magic_number = file.read_u32::<LittleEndian>()?;
        if magic_number != MAGIC_NUMBER {
            return Err(anyhow::anyhow!(
                "This seems not to be RDF Glance file. Wrong magic number",
            ));
        }
        let _version = file.read_u16::<LittleEndian>()?;
        let _flags =  file.read_u16::<LittleEndian>()?;
        let header_length = file.read_u16::<LittleEndian>()?;
        println!("header lenght {}",header_length);
        file.seek(SeekFrom::Start(header_length as u64))?;

        loop {
            match file.read_u8() {
                Ok(header_type_u8) => {
                    println!("reading header type {}",header_type_u8);
                    let header_type = HeaderType::from_u8(header_type_u8);
                    let block_size = file.read_u32::<LittleEndian>()?;
                    println!("block size {}", block_size);
                    if let Some(header_type) = header_type {
                        match header_type {
                            HeaderType::DataTypes => {
                                app.node_data.indexers.datatype_indexer = StringIndexer::restore(&mut file, block_size-5)?;
                            }
                            HeaderType::Languages => {
                                app.node_data.indexers.language_indexer = StringIndexer::restore(&mut file, block_size-5)?;
                            }
                            HeaderType::Predicates => {
                                app.node_data.indexers.predicate_indexer = StringIndexer::restore(&mut file, block_size-5)?;
                            }
                            HeaderType::Types => {
                                app.node_data.indexers.type_indexer = StringIndexer::restore(&mut file, block_size-5)?;
                            }
                            HeaderType::Nodes => {
                                app.node_data.node_cache = NodeCache::restore(&mut file, block_size-5)?;
                            }
                        }
                    } else {
                        println!("unknown header type {} ignoring block",header_type_u8);
                        file.seek(SeekFrom::Current((block_size - 5) as i64))?;
                    }
                },
                Err(_) => {
                    println!("End of file");
                    break;
                }
            }
        }


        return Ok(app);
    }
}

fn with_header_len(file: &mut BufWriter<File>, header_type: u8, f: &dyn Fn(&mut BufWriter<File>) -> std::io::Result<()>) -> std::io::Result<()> {
    file.write_u8(header_type)?;
    let size_pos = file.seek(SeekFrom::Current(0))?;
    file.write_u32::<LittleEndian>(0)?; // Placeholder for size
    f(file)?;
    let end_pos = file.seek(SeekFrom::Current(0))?;
    let size = end_pos - size_pos + 1;
    file.seek(SeekFrom::Start(size_pos))?;
    file.write_u32::<LittleEndian>(size as u32)?;
    file.seek(SeekFrom::End(0))?;
    Ok(())
}

impl StringIndexer {
    pub fn store(&self, header_type: u8, file: &mut BufWriter<File>) -> std::io::Result<()> {
        let len = self.map.len();
        let reverse: HashMap<IriIndex, &String> = self.iter()
            .map(|(key, &value)| (value, key))
            .collect();
        with_header_len(file, header_type, &|file| {
            for i in 0..len {
                let i : IriIndex = i as IriIndex;
                let elem = reverse.get(&i);
                if let Some(elem) = elem {
                    file.write_all(elem.as_bytes())?;
                    file.write_u8(0x1F)?;
                }
            }
            Ok(())
        })
    }

    pub fn restore(file: &mut File, size: u32) -> Result<Self> {
        let mut index = Self::new();
        let mut idx_num: IriIndex = 0;
        let mut buffer: Vec<u8> = Vec::with_capacity(256);
        for _i in 0..size {
            let byte = file.read_u8()?;
            if byte == 0x1F {
                let str = std::str::from_utf8(&buffer)?;
                index.map.insert(str.to_owned(), idx_num);
                buffer.clear();
                idx_num += 1;
            } else {
                buffer.push(byte);
                match byte {
                    0x00..=0x7F => {
                    }
                    0xC0..=0xDF => {
                        buffer.push(file.read_u8()?);
                    }
                    0xE0..=0xEF => {
                        buffer.push(file.read_u8()?);
                        buffer.push(file.read_u8()?);
                    }
                    0xF0..=0xF7 => {
                        buffer.push(file.read_u8()?);
                        buffer.push(file.read_u8()?);
                        buffer.push(file.read_u8()?);   
                    }
                    _ => {
                        println!("Invalid UTF-8 byte detected: 0x{:X}", byte);
                    }
                };
            }
        }
        return Ok(index);
    }
}

impl NodeCache {
    pub fn store(&self, header_type: u8, file: &mut BufWriter<File>) -> std::io::Result<()> {
        Ok(())
    }

    pub fn restore(file: &mut File, size: u32) -> Result<Self> {
        let cache = NodeCache::new();
        return Ok(cache);
    }
}


#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;

    fn get_test_file_path(filename: &str) -> PathBuf {
        let mut dir = PathBuf::from("target/test-files");
        fs::create_dir_all(&dir).expect("Failed to create test directory"); // Ensure directory exists
        dir.push(filename);
        dir
    }
    

    #[test]
    fn test_store()  -> std::io::Result<()> {
        let store_path = get_test_file_path("store.rdfglance");
        
        let mut vs = RdfGlanceApp::new(None);
        vs.load_ttl("sample-rdf-data/programming_languages.ttl");
        println!("nodes read {}",vs.node_data.len());
        vs.store(&store_path)?;

        assert!(store_path.exists(),"file does not exists");
        let mut restored = RdfGlanceApp::restore(&store_path).unwrap();

        assert!(vs.node_data.indexers.datatype_indexer.map.len()==restored.node_data.indexers.datatype_indexer.map.len());
        assert!(vs.node_data.indexers.language_indexer.map.len()==restored.node_data.indexers.language_indexer.map.len());
        assert!(vs.node_data.indexers.predicate_indexer.map.len()==restored.node_data.indexers.predicate_indexer.map.len());
        assert!(vs.node_data.indexers.type_indexer.map.len()==restored.node_data.indexers.type_indexer.map.len());

        let predicates : Vec<String> = vs.node_data.indexers.predicate_indexer.map.keys().cloned().collect();
        for pred_val in &predicates {
            assert!(vs.node_data.indexers.get_predicate_index(pred_val)==restored.node_data.indexers.get_predicate_index(pred_val))
        }

        Ok(())
    }
   
}

