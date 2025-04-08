use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use anyhow::Result;
use leb128;

use crate::nobject::{DataTypeIndex, IriIndex, LangIndex, Literal, NObject, NodeCache, PredicateLiteral, StringIndexer};
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

        self.node_data.indexers.predicate_indexer.store(HeaderType::Predicates, &mut file)?;
        self.node_data.indexers.type_indexer.store(HeaderType::Types, &mut file)?;
        self.node_data.indexers.language_indexer.store(HeaderType::Languages, &mut file)?;
        self.node_data.indexers.datatype_indexer.store(HeaderType::DataTypes, &mut file)?;
        self.node_data.node_cache.store(&mut file)?;

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
        Ok(app)
    }
}

fn with_header_len(file: &mut BufWriter<File>, header_type: HeaderType, f: &dyn Fn(&mut BufWriter<File>) -> std::io::Result<()>) -> std::io::Result<()> {
    file.write_u8(header_type as u8)?;
    let size_pos = file.stream_position()?;
    file.write_u32::<LittleEndian>(0)?; // Placeholder for size
    f(file)?;
    let end_pos = file.stream_position()?;
    let size = end_pos - size_pos + 1;
    file.seek(SeekFrom::Start(size_pos))?;
    file.write_u32::<LittleEndian>(size as u32)?;
    file.seek(SeekFrom::End(0))?;
    Ok(())
}

impl StringIndexer {
    pub fn store(&self, header_type: HeaderType, file: &mut BufWriter<File>) -> std::io::Result<()> {
        let len = self.map.len();
        let reverse: HashMap<IriIndex, &Box<str>> = self.iter()
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
                index.map.insert(str.into(), idx_num);
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
        Ok(index)
    }
}

impl NodeCache {
    pub fn store(&self, file: &mut BufWriter<File>) -> std::io::Result<()> {
        with_header_len(file, HeaderType::Nodes, &|file| {
            leb128::write::unsigned(file, self.cache.len() as u64)?;
            for (iri,node) in self.iter() {
                write_len_string(iri, file)?;
                let flags: u8 = if node.is_blank_node { 1 } else { 0 } | if node.has_subject { 2 } else { 0};
                file.write_u8(flags)?;
                leb128::write::unsigned(file, node.types.len() as u64)?;
                for type_index in node.types.iter() {
                    leb128::write::unsigned(file, *type_index as u64)?; 
                }
                leb128::write::unsigned(file, node.properties.len() as u64)?;
                for (predicate_index, literal) in node.properties.iter() {
                    leb128::write::unsigned(file, *predicate_index as u64)?; 
                    literal.store(file)?;
                }
                leb128::write::unsigned(file, node.references.len() as u64)?; 
                for (predicate_index, iri_index) in node.references.iter() {
                    leb128::write::unsigned(file, *predicate_index as u64)?; 
                    leb128::write::unsigned(file, *iri_index as u64)?; 
                }
                leb128::write::unsigned(file, node.reverse_references.len() as u64)?; 
                for (predicate_index, iri_index) in node.reverse_references.iter() {
                    leb128::write::unsigned(file, *predicate_index as u64)?; 
                    leb128::write::unsigned(file, *iri_index as u64)?; 
                }
            }
            Ok(())
        })
    }

    pub fn restore(file: &mut File, _size: u32) -> Result<Self> {
        let mut cache = NodeCache::new();
        let nodes_len = leb128::read::unsigned(file)?;
        println!("read {} nodes",nodes_len);
        for _ in 0..nodes_len {
            let iri = read_len_string(file)?;
            println!("read node with iri {}",iri);
            let flags = file.read_u8()?;
            let is_blank_node = (flags & 1)>0;
            let has_subject = (flags & 2)>0;
            let types_len = leb128::read::unsigned(file)?;
            let mut types: Vec<IriIndex> = Vec::with_capacity(types_len as usize);
            for _ in 0..types_len {
                let type_index = leb128::read::unsigned(file)? as IriIndex;
                types.push(type_index);
            }
            let properties_len = leb128::read::unsigned(file)?;
            let mut properties: Vec<PredicateLiteral> = Vec::with_capacity(types_len as usize);
            for _ in 0..properties_len {
                let predicate_index = leb128::read::unsigned(file)? as IriIndex;
                let literal = Literal::restore(file)?;
                properties.push((predicate_index,literal));
            }
            let references_len =  leb128::read::unsigned(file)?;
            let mut references: Vec<(IriIndex,IriIndex)> = Vec::with_capacity(types_len as usize);
            for _ in 0..references_len {
                let predicate_index = leb128::read::unsigned(file)? as IriIndex;
                let iri_index = leb128::read::unsigned(file)? as IriIndex;
                references.push((predicate_index,iri_index));
            }
            let reverse_references_len =  leb128::read::unsigned(file)?;
            let mut reverse_references: Vec<(IriIndex,IriIndex)> = Vec::with_capacity(types_len as usize);
            for _ in 0..reverse_references_len {
                let predicate_index = leb128::read::unsigned(file)? as IriIndex;
                let iri_index = leb128::read::unsigned(file)? as IriIndex;
                reverse_references.push((predicate_index,iri_index));
            }
            let node = NObject {
                types,
                properties,
                references,
                reverse_references,
                is_blank_node,
                has_subject
            };
            cache.cache.insert(iri.into(),node);
        }
        Ok(cache)
    }
}

fn read_len_string(file: &mut File) -> Result<Box<str>> {
    let str_len = leb128::read::unsigned(file)?;
    let mut buffer = vec![0; str_len as usize];
    file.read_exact(&mut buffer)?;
    let str = std::str::from_utf8(&buffer)?;
    Ok(str.into())
}

fn write_len_string(str: &str, file: &mut BufWriter<File>) -> std::io::Result<()> {
    let iri_bytes = str.as_bytes();
    leb128::write::unsigned(file, iri_bytes.len() as u64)?; 
    file.write_all(iri_bytes)?;
    Ok(())
}

impl Literal {
    pub fn store(&self, file: &mut BufWriter<File>)  -> std::io::Result<()> {
        match self {
            Literal::String(str) => {
                file.write_u8(1)?;
                write_len_string(str, file)?;
            },
            Literal::LangString(lang_index, str) => {
                file.write_u8(2)?;
                leb128::write::unsigned(file, *lang_index as u64)?;
                write_len_string(str, file)?;
            },
            Literal::TypedString(type_index, str) => {
                file.write_u8(3)?;
                leb128::write::unsigned(file, *type_index as u64)?;
                write_len_string(str, file)?;
            }
        }
        Ok(())
    }
    pub fn restore(file: &mut File) -> Result<Self> {
        let literal_type = file.read_u8()?;
        match literal_type {
            1 => {
                Ok(Literal::String(read_len_string(file)?))
            },
            2 => {
                let lang_index = leb128::read::unsigned(file)? as LangIndex;
                Ok(Literal::LangString(lang_index,read_len_string(file)?))
            },
            3 => {
                let data_type_index = leb128::read::unsigned(file)? as DataTypeIndex;
                Ok(Literal::TypedString(data_type_index,read_len_string(file)?))
            },
            _ => {
                Err(anyhow::anyhow!(
                    "Unknown literal type {}",literal_type
                ))
            }
        }
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

        assert_eq!(vs.node_data.indexers.datatype_indexer.map.len(),restored.node_data.indexers.datatype_indexer.map.len());
        assert_eq!(vs.node_data.indexers.language_indexer.map.len(),restored.node_data.indexers.language_indexer.map.len());
        assert_eq!(vs.node_data.indexers.predicate_indexer.map.len(),restored.node_data.indexers.predicate_indexer.map.len());
        assert_eq!(vs.node_data.indexers.type_indexer.map.len(),restored.node_data.indexers.type_indexer.map.len());

        let predicates : Vec<Box<str>> = vs.node_data.indexers.predicate_indexer.map.keys().cloned().collect();
        for pred_val in &predicates {
            assert!(vs.node_data.indexers.get_predicate_index(pred_val)==restored.node_data.indexers.get_predicate_index(pred_val))
        }

        assert_eq!(vs.node_data.len(),restored.node_data.len());

        Ok(())
    }
   
}

