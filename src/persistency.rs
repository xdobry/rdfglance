use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use anyhow::Result;
use egui::Vec2;
use flate2::read::ZlibDecoder;
use leb128;
use flate2::Compression;
use flate2::write::ZlibEncoder;

use crate::layout::{NodeLayout, SortedNodeLayout};
use crate::nobject::{DataTypeIndex, IriIndex, LangIndex, Literal, NObject, NodeCache, PredicateLiteral, StringIndexer};
use crate::prefix_manager::PrefixManager;
use crate::{GVisualisationStyle, RdfGlanceApp};

// it is just ascii "rdfg"
const MAGIC_NUMBER: u32 = 0x47464452;
const FORMAT_VERSION: u16 = 0;
const FORMAT_FLAGS: u16 = 0;

const BLOCK_PRELUDE_SIZE: u32 = 5;

#[repr(u8)]
pub enum HeaderType {
    Predicates = 0x01,
    Types = 0x02,
    Languages = 0x03,
    DataTypes = 0x04,
    Nodes = 0x05,
    Prefixes = 0x06,
    VisualNodes = 0x07,
    VisualStyles = 0x08,
}

impl HeaderType {
    pub fn from_u8(value: u8) -> Option<HeaderType> {
        match value {
            1 => Some(HeaderType::Predicates),
            2 => Some(HeaderType::Types),
            3 => Some(HeaderType::Languages),
            4 => Some(HeaderType::DataTypes),
            5 => Some(HeaderType::Nodes),
            6 => Some(HeaderType::Prefixes),
            7 => Some(HeaderType::VisualNodes),
            8 => Some(HeaderType::VisualStyles),
            _ => None,
        }
    }
}

#[repr(u8)]
pub enum FieldType {
    VARINT = 0x01,
    FIX64 = 0x02,
    FIX32 = 0x03,
    LENGTHDELIMITED = 0x04,    
}

impl FieldType {
    pub fn from_idx(value: u8) -> Result<Self> {
        match value {
            1 => Ok(FieldType::VARINT),
            2 => Ok(FieldType::FIX64),
            3 => Ok(FieldType::FIX32),
            4 => Ok(FieldType::LENGTHDELIMITED),
            _ => Err(anyhow::anyhow!("Unknown field type")),
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
        self.prefix_manager.store(&mut file)?;
        self.ui_state.visible_nodes.store(&mut file)?;
        self.visualisation_style.store(&mut file)?;

        file.flush()?;
        Ok(())
    }

    pub fn restore(path: &Path) -> Result<Self> {
        let mut app = RdfGlanceApp::new(None);
        let file = File::open(path)?;
        let mut reader = BufReader::new(&file);
        let magic_number = reader.read_u32::<LittleEndian>()?;
        if magic_number != MAGIC_NUMBER {
            return Err(anyhow::anyhow!(
                "This seems not to be RDF Glance file. Wrong magic number",
            ));
        }
        let _version = reader.read_u16::<LittleEndian>()?;
        let _flags =  reader.read_u16::<LittleEndian>()?;
        let header_length = reader.read_u16::<LittleEndian>()?;
        println!("header lenght {}",header_length);
        reader.seek(SeekFrom::Start(header_length as u64))?;

        loop {
            match reader.read_u8() {
                Ok(header_type_u8) => {
                    println!("reading header type {}",header_type_u8);
                    let header_type = HeaderType::from_u8(header_type_u8);
                    let block_size = reader.read_u32::<LittleEndian>()?;
                    println!("block size {}", block_size);
                    if let Some(header_type) = header_type {
                        match header_type {
                            HeaderType::DataTypes => {
                                app.node_data.indexers.datatype_indexer = StringIndexer::restore(&mut reader, block_size-BLOCK_PRELUDE_SIZE)?;
                            }
                            HeaderType::Languages => {
                                app.node_data.indexers.language_indexer = StringIndexer::restore(&mut reader, block_size-BLOCK_PRELUDE_SIZE)?;
                            }
                            HeaderType::Predicates => {
                                app.node_data.indexers.predicate_indexer = StringIndexer::restore(&mut reader, block_size-BLOCK_PRELUDE_SIZE)?;
                            }
                            HeaderType::Types => {
                                app.node_data.indexers.type_indexer = StringIndexer::restore(&mut reader, block_size-BLOCK_PRELUDE_SIZE)?;
                            }
                            HeaderType::Nodes => {
                                app.node_data.node_cache = NodeCache::restore(&mut reader, block_size-BLOCK_PRELUDE_SIZE)?;
                            }
                            HeaderType::Prefixes => {
                                app.prefix_manager = PrefixManager::restore(&mut reader, block_size-BLOCK_PRELUDE_SIZE)?;
                            }
                            HeaderType::VisualNodes => {
                                app.ui_state.visible_nodes = SortedNodeLayout::restore(&mut reader, block_size-BLOCK_PRELUDE_SIZE)?;
                            }
                            HeaderType::VisualStyles => {
                                app.visualisation_style = GVisualisationStyle::restore(&mut reader, block_size-BLOCK_PRELUDE_SIZE)?;
                            }
                        }
                    } else {
                        println!("unknown header type {} ignoring block",header_type_u8);
                        reader.seek(SeekFrom::Current((block_size - 5) as i64))?;
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
    pub fn store(&self, header_type: HeaderType, writer: &mut BufWriter<File>) -> std::io::Result<()> {
        with_header_len(writer, header_type, &|file| {
            let mut compressor = ZlibEncoder::new( file, Compression::default());
            for (_index, lang) in self.map.iter() {
                compressor.write_all(lang.as_bytes())?;
                compressor.write_u8(0x1F)?;
            }
            compressor.finish()?;
            Ok(())
        })
    }

    pub fn restore<R: Read>(reader: &mut R, size: u32) -> Result<Self> {
        let mut index = Self::new();
        let mut buffer: Vec<u8> = Vec::with_capacity(256);
        let limited_reader = reader.by_ref().take(size as u64);
        let mut decoder = ZlibDecoder::new(limited_reader);
        let mut byte = [0u8; 1];
        loop {
            let read = decoder.read(&mut byte)?;
            if read == 0 {
                break;
            }
            if byte[0] == 0x1F {
                let str = std::str::from_utf8(&buffer)?;
                index.map.get_or_intern(str);
                buffer.clear();
            } else {
                buffer.push(byte[0]);
                match byte[0] {
                    0x00..=0x7F => {
                    }
                    0xC0..=0xDF => {
                        let read = decoder.read(&mut byte)?;
                        if read == 0 {
                            println!("Expect 1 addtional byte for utf8");
                            break;
                        }
                        buffer.push(byte[0]);
                    }
                    0xE0..=0xEF => {
                        let mut byte2 = [0u8; 2];
                        let read = decoder.read(&mut byte2)?;
                        if read != 2 {
                            println!("Expect 2 addtional bytes for utf8");
                            break;
                        }
                        buffer.extend_from_slice(&byte2[0..2]);
                    }
                    0xF0..=0xF7 => {
                        let mut byte3 = [0u8; 3];
                        let read = decoder.read(&mut byte3)?;
                        if read != 3 {
                            println!("Expect 3 addtional bytes for utf8");
                            break;
                        }
                        buffer.extend_from_slice(&byte3[0..3]);
                    }
                    _ => {
                        println!("Invalid UTF-8 byte detected: 0x{:X}", byte[0]);
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

    pub fn restore<R: Read>(reader: &mut R, _size: u32) -> Result<Self> {
        let mut cache = NodeCache::new();
        let nodes_len = leb128::read::unsigned(reader)?;
        // println!("read {} nodes",nodes_len);
        for _ in 0..nodes_len {
            let iri = read_len_string(reader)?;
            // println!("read node with iri {}",iri);
            let flags = reader.read_u8()?;
            let is_blank_node = (flags & 1)>0;
            let has_subject = (flags & 2)>0;
            let types_len = leb128::read::unsigned(reader)?;
            let mut types: Vec<IriIndex> = Vec::with_capacity(types_len as usize);
            for _ in 0..types_len {
                let type_index = leb128::read::unsigned(reader)? as IriIndex;
                types.push(type_index);
            }
            let properties_len = leb128::read::unsigned(reader)?;
            let mut properties: Vec<PredicateLiteral> = Vec::with_capacity(types_len as usize);
            for _ in 0..properties_len {
                let predicate_index = leb128::read::unsigned(reader)? as IriIndex;
                let literal = Literal::restore(reader)?;
                properties.push((predicate_index,literal));
            }
            let references_len =  leb128::read::unsigned(reader)?;
            let mut references: Vec<(IriIndex,IriIndex)> = Vec::with_capacity(types_len as usize);
            for _ in 0..references_len {
                let predicate_index = leb128::read::unsigned(reader)? as IriIndex;
                let iri_index = leb128::read::unsigned(reader)? as IriIndex;
                references.push((predicate_index,iri_index));
            }
            let reverse_references_len =  leb128::read::unsigned(reader)?;
            let mut reverse_references: Vec<(IriIndex,IriIndex)> = Vec::with_capacity(types_len as usize);
            for _ in 0..reverse_references_len {
                let predicate_index = leb128::read::unsigned(reader)? as IriIndex;
                let iri_index = leb128::read::unsigned(reader)? as IriIndex;
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

fn read_len_string<R: Read>(reader: &mut R) -> Result<Box<str>> {
    let str_len = leb128::read::unsigned(reader)?;
    let mut buffer = vec![0; str_len as usize];
    reader.read_exact(&mut buffer)?;
    let str = std::str::from_utf8(&buffer)?;
    Ok(str.into())
}

fn write_len_string<W: Write>(str: &str, writer: &mut W) -> std::io::Result<()> {
    let iri_bytes = str.as_bytes();
    leb128::write::unsigned(writer, iri_bytes.len() as u64)?; 
    writer.write_all(iri_bytes)?;
    Ok(())
}

fn write_field_index<W: Write>(writer: &mut W, field_type: FieldType, field_index: u32) -> std::io::Result<()> {
    let field_encoded = (field_index<<3) as u64 | field_type as u64;
    leb128::write::unsigned(writer, field_encoded)?; 
    Ok(())
}

fn read_field_index<R: Read>(reader: &mut R) -> Result<(FieldType, u32)> {
    let field_encoded = leb128::read::unsigned(reader)?;
    let field_type_idx = (field_encoded & 0x7) as u8;
    let field_index = (field_encoded >> 3) as u32;
    let field_type = FieldType::from_idx(field_type_idx)?;
    Ok((field_type, field_index))
}

fn skip_field(reader: &mut BufReader<&File>, field_type: FieldType) -> Result<()> {
    match field_type {
        FieldType::VARINT => {
            let _len = leb128::read::unsigned(reader)?;
        }
        FieldType::FIX64 => {
            let _value = reader.read_u64::<LittleEndian>();
        }
        FieldType::FIX32 => {
            let _value = reader.read_u32::<LittleEndian>();
        }
        FieldType::LENGTHDELIMITED => {
            let len = leb128::read::unsigned(reader)?;
            reader.seek(SeekFrom::Current(len as i64))?;
        }
    }
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
    pub fn restore<R: Read>(reader: &mut R) -> Result<Self> {
        let literal_type = reader.read_u8()?;
        match literal_type {
            1 => {
                Ok(Literal::String(read_len_string(reader)?))
            },
            2 => {
                let lang_index = leb128::read::unsigned(reader)? as LangIndex;
                Ok(Literal::LangString(lang_index,read_len_string(reader)?))
            },
            3 => {
                let data_type_index = leb128::read::unsigned(reader)? as DataTypeIndex;
                Ok(Literal::TypedString(data_type_index,read_len_string(reader)?))
            },
            _ => {
                Err(anyhow::anyhow!(
                    "Unknown literal type {}",literal_type
                ))
            }
        }
    }
}

impl PrefixManager {
    pub fn store(&self, writer: &mut BufWriter<File>) -> std::io::Result<()> {
        let len = self.prefixes.len();
        with_header_len(writer, HeaderType::Prefixes, &|file| {
            file.write_u32::<LittleEndian>(len as u32)?;
            {
                let mut compressor = ZlibEncoder::new( file, Compression::default());
                for (iri, prefix) in self.prefixes.iter() {
                    write_len_string(&prefix, &mut compressor)?;
                    write_len_string(&iri, &mut compressor)?;
                }
                compressor.finish()?;
            }
            Ok(())
        })
    }

    pub fn restore<R: Read>(reader: &mut R, size: u32) -> Result<Self> {
        let mut index = Self::new();
        let len = reader.read_u32::<LittleEndian>()?;
        let limited_reader = reader.by_ref().take((size-4) as u64);
        let mut decoder = ZlibDecoder::new(limited_reader);
        for _ in 0..len {
            let prefix = read_len_string(&mut decoder)?;
            let iri = read_len_string(&mut decoder)?;
            index.prefixes.insert(iri.into(), prefix.into());
        }
        Ok(index)
    }    
}

impl SortedNodeLayout {
    pub fn store(&self, writer: &mut BufWriter<File>) -> std::io::Result<()> {
        with_header_len(writer, HeaderType::VisualNodes, &|writer| {
            leb128::write::unsigned(writer, self.nodes.len() as u64)?;
            for node_layout in self.nodes.iter() {
                leb128::write::unsigned(writer, node_layout.node_index as u64)?;
                writer.write_f32::<LittleEndian>(node_layout.pos.x)?;
                writer.write_f32::<LittleEndian>(node_layout.pos.y)?;
                // Write number of fields
                leb128::write::unsigned(writer, 0)?; 
            }
            Ok(())
        })
    }

    pub fn restore(reader: &mut BufReader<&File>, _size: u32) -> Result<Self> {
        let len = leb128::read::unsigned(reader)?;
        let mut index = SortedNodeLayout { nodes: Vec::with_capacity(len as usize) };
        for _ in 0..len {
            let node_index = leb128::read::unsigned(reader)? as IriIndex;
            let x = reader.read_f32::<LittleEndian>()?;
            let y = reader.read_f32::<LittleEndian>()?;
            let field_number = leb128::read::unsigned(reader)?;
            for _ in 0..field_number {
                let (field_type, _field_index) = read_field_index(reader)?;
                skip_field(reader, field_type)?;
            }
            index.nodes.push(NodeLayout {
                node_index,
                pos: egui::Pos2::new(x, y),
                vel: Vec2::new(0.0, 0.0),
            });            
        }
        Ok(index)
    }
}

impl GVisualisationStyle {
    pub fn store(&self, writer: &mut BufWriter<File>) -> std::io::Result<()> {
        with_header_len(writer, HeaderType::VisualStyles, &|writer| {
            leb128::write::unsigned(writer, self.type_styles.len() as u64)?;
            for (type_index, style) in self.type_styles.iter() {
                leb128::write::unsigned(writer, *type_index as u64)?;
                leb128::write::unsigned(writer, style.label_index as u64)?;
                leb128::write::unsigned(writer, style.priority as u64)?;
                let col = style.color.to_array();
                writer.write(&col)?;
                leb128::write::unsigned(writer, 0)?;
            }
            leb128::write::unsigned(writer, self.reference_styles.len() as u64)?;
            for (reference_index, style) in self.reference_styles.iter() {
                leb128::write::unsigned(writer, *reference_index as u64)?;
                let col = style.color.to_array();
                writer.write(&col)?;
                leb128::write::unsigned(writer, 0)?;
            }
            Ok(())
        })
    }

    pub fn restore(reader: &mut BufReader<&File>, _size: u32) -> Result<Self> {
        let mut styles = GVisualisationStyle {
            type_styles: HashMap::new(),
            reference_styles: HashMap::new(),
        };
        let len_types = leb128::read::unsigned(reader)?;
        for _ in 0..len_types {
            let type_index = leb128::read::unsigned(reader)? as IriIndex;
            let label_index = leb128::read::unsigned(reader)? as IriIndex;
            let priority = leb128::read::unsigned(reader)? as u32;
            let mut color = [0u8; 4];
            reader.read_exact(&mut color)?;
            let field_number = leb128::read::unsigned(reader)?;
            for _ in 0..field_number {
                let (field_type, _field_index) = read_field_index(reader)?;
                skip_field(reader, field_type)?;
            }
            let style = crate::TypeStyle { 
                color: egui::Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]),
                priority: priority, 
                label_index: label_index, 
            };
            styles.type_styles.insert(type_index, style);
        }
        let len_references = leb128::read::unsigned(reader)?;
        for _ in 0..len_references {
            let reference_index = leb128::read::unsigned(reader)? as IriIndex;
            let mut color = [0u8; 4];
            reader.read_exact(&mut color)?;
            let style = crate::ReferenceStyle { 
                color: egui::Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]),
            };
            let field_number = leb128::read::unsigned(reader)?;
            for _ in 0..field_number {
                let (field_type, _field_index) = read_field_index(reader)?;
                skip_field(reader, field_type)?;
            }
            styles.reference_styles.insert(reference_index, style);
        }

        Ok(styles)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::Instant};

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
        let start = Instant::now();
        vs.load_ttl("sample-rdf-data/programming_languages.ttl");
        println!("nodes read {}",vs.node_data.len());
        let duration = start.elapsed();
        println!("Time taken to read ttl {:?}", duration);

        let label_index = vs.node_data.indexers.predicate_indexer.get_index("rdfs:label");
        assert_eq!(0, label_index);

        vs.store(&store_path)?;

        assert!(store_path.exists(),"file does not exists");
        let start = Instant::now();
        let mut restored = RdfGlanceApp::restore(&store_path).unwrap();
        let duration = start.elapsed();
        println!("Time taken to read project {:?}", duration);

        assert_eq!(vs.node_data.indexers.datatype_indexer.map.len(),restored.node_data.indexers.datatype_indexer.map.len());
        assert_eq!(vs.node_data.indexers.language_indexer.map.len(),restored.node_data.indexers.language_indexer.map.len());
        assert_eq!(vs.node_data.indexers.predicate_indexer.map.len(),restored.node_data.indexers.predicate_indexer.map.len());
        assert_eq!(vs.node_data.indexers.type_indexer.map.len(),restored.node_data.indexers.type_indexer.map.len());

        let predicates = vec!["rdf:type"];
        for pred_val in &predicates {
            assert!(vs.node_data.indexers.get_predicate_index(pred_val)==restored.node_data.indexers.get_predicate_index(pred_val))
        }

        assert_eq!(vs.node_data.len(),restored.node_data.len());
        assert_eq!(vs.prefix_manager.prefixes.len(),restored.prefix_manager.prefixes.len());
        assert_eq!(vs.ui_state.visible_nodes.nodes.len(),restored.ui_state.visible_nodes.nodes.len());    
        assert_eq!(vs.visualisation_style.type_styles.len(),restored.visualisation_style.type_styles.len());    

        Ok(())
    }
   
}

