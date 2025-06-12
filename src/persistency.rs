use anyhow::{Error, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use egui::{Pos2, Vec2};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use leb128;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::graph_styles::{
    ArrowLocation, ArrowStyle, EdgeFont, IconStyle, LabelPosition, LineStyle, NodeShape, NodeSize,
};
use crate::layout::{NodeLayout, NodePosition, NodeShapeData, SortedNodeLayout};
use crate::nobject::{DataTypeIndex, IriIndex, LangIndex, Literal, NObject, NodeCache, PredicateLiteral};
use crate::prefix_manager::PrefixManager;
use crate::string_indexer::{IndexSpan, StringCache, StringIndexer};
use crate::{EdgeStyle, GVisualisationStyle, RdfGlanceApp};

// it is just ascii "rdfg"
const MAGIC_NUMBER: u32 = 0x47464452;
const FORMAT_VERSION: u16 = 0;
const FORMAT_FLAGS: u16 = 0;

const BLOCK_PRELUDE_SIZE: u32 = 5;

#[repr(u8)]
pub enum HeaderType {
    Predicates = 1,
    Types = 2,
    Languages = 3,
    DataTypes = 4,
    Nodes = 5,
    Prefixes = 6,
    VisualNodes = 7,
    VisualStyles = 8,
    Literals = 9,
    ShortLiterals = 10,
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
            9 => Some(HeaderType::Literals),
            10 => Some(HeaderType::ShortLiterals),
            _ => None,
        }
    }
}

#[repr(u8)]
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum FieldType {
    VARINT = 0x01,
    FIX64 = 0x02,
    FIX32 = 0x03,
    LENGTHDELIMITED = 0x04,
    FLAG = 0x05,
}

impl FieldType {
    pub fn from_idx(value: u8) -> Result<Self> {
        match value {
            1 => Ok(FieldType::VARINT),
            2 => Ok(FieldType::FIX64),
            3 => Ok(FieldType::FIX32),
            4 => Ok(FieldType::LENGTHDELIMITED),
            5 => Ok(FieldType::FLAG),
            _ => Err(anyhow::anyhow!("Unknown field type")),
        }
    }
}

impl RdfGlanceApp {
    pub fn store(&self, path: &Path) -> std::io::Result<()> {
        let mut file = BufWriter::new(File::create(path)?);

        file.write_u32::<LittleEndian>(MAGIC_NUMBER)?;
        file.write_u16::<LittleEndian>(FORMAT_VERSION)?;
        file.write_u16::<LittleEndian>(FORMAT_FLAGS)?;
        // header size
        file.write_u16::<LittleEndian>(10)?;

        if let Ok(rdf_data) = self.rdf_data.read() {
            rdf_data
                .node_data
                .indexers
                .predicate_indexer
                .store(HeaderType::Predicates, &mut file)?;
            rdf_data
                .node_data
                .indexers
                .type_indexer
                .store(HeaderType::Types, &mut file)?;
            rdf_data
                .node_data
                .indexers
                .language_indexer
                .store(HeaderType::Languages, &mut file)?;
            rdf_data
                .node_data
                .indexers
                .datatype_indexer
                .store(HeaderType::DataTypes, &mut file)?;
            rdf_data
                .node_data
                .indexers
                .short_literal_indexer
                .store(HeaderType::ShortLiterals, &mut file)?;
            rdf_data.node_data.indexers.literal_cache.store(&mut file)?;
            rdf_data.node_data.node_cache.store(&mut file)?;
            rdf_data.prefix_manager.store(&mut file)?;
        }
        self.visible_nodes.store(&mut file)?;
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
        let _flags = reader.read_u16::<LittleEndian>()?;
        let header_length = reader.read_u16::<LittleEndian>()?;
        println!("header lenght {}", header_length);
        reader.seek(SeekFrom::Start(header_length as u64))?;

        loop {
            match reader.read_u8() {
                Ok(header_type_u8) => {
                    println!("reading header type {}", header_type_u8);
                    let header_type = HeaderType::from_u8(header_type_u8);
                    let block_size = reader.read_u32::<LittleEndian>()?;
                    println!("block size {}", block_size);
                    if let Some(header_type) = header_type {
                        match header_type {
                            HeaderType::DataTypes => {
                                app.mut_rdf_data(|rdf_data| {
                                    rdf_data.node_data.indexers.datatype_indexer =
                                        StringIndexer::restore(&mut reader, block_size - BLOCK_PRELUDE_SIZE)?;
                                    Ok::<(), Error>(())
                                })
                                .unwrap()?;
                            }
                            HeaderType::Languages => {
                                app.mut_rdf_data(|rdf_data| {
                                    rdf_data.node_data.indexers.language_indexer =
                                        StringIndexer::restore(&mut reader, block_size - BLOCK_PRELUDE_SIZE)?;
                                    Ok::<(), Error>(())
                                })
                                .unwrap()?;
                            }
                            HeaderType::Predicates => {
                                app.mut_rdf_data(|rdf_data| {
                                    rdf_data.node_data.indexers.predicate_indexer =
                                        StringIndexer::restore(&mut reader, block_size - BLOCK_PRELUDE_SIZE)?;
                                    Ok::<(), Error>(())
                                })
                                .unwrap()?;
                            }
                            HeaderType::Types => {
                                app.mut_rdf_data(|rdf_data| {
                                    rdf_data.node_data.indexers.type_indexer =
                                        StringIndexer::restore(&mut reader, block_size - BLOCK_PRELUDE_SIZE)?;
                                    Ok::<(), Error>(())
                                })
                                .unwrap()?;
                            }
                            HeaderType::Nodes => {
                                app.mut_rdf_data(|rdf_data| {
                                    rdf_data.node_data.node_cache =
                                        NodeCache::restore(&mut reader, block_size - BLOCK_PRELUDE_SIZE)?;
                                    Ok::<(), Error>(())
                                })
                                .unwrap()?;
                            }
                            HeaderType::Prefixes => {
                                app.mut_rdf_data(|rdf_data| {
                                    rdf_data.prefix_manager =
                                        PrefixManager::restore(&mut reader, block_size - BLOCK_PRELUDE_SIZE)?;
                                    Ok::<(), Error>(())
                                })
                                .unwrap()?;
                            }
                            HeaderType::VisualNodes => {
                                app.visible_nodes =
                                    SortedNodeLayout::restore(&mut reader, block_size - BLOCK_PRELUDE_SIZE)?;
                            }
                            HeaderType::VisualStyles => {
                                app.visualisation_style =
                                    GVisualisationStyle::restore(&mut reader, block_size - BLOCK_PRELUDE_SIZE)?;
                            }
                            HeaderType::Literals => {
                                app.mut_rdf_data(|rdf_data| {
                                    rdf_data.node_data.indexers.literal_cache =
                                        StringCache::restore(&mut reader, block_size - BLOCK_PRELUDE_SIZE)?;
                                    Ok::<(), Error>(())
                                })
                                .unwrap()?;
                            }
                            HeaderType::ShortLiterals => {
                                app.mut_rdf_data(|rdf_data| {
                                    rdf_data.node_data.indexers.short_literal_indexer =
                                        StringIndexer::restore(&mut reader, block_size - BLOCK_PRELUDE_SIZE)?;
                                    Ok::<(), Error>(())
                                })
                                .unwrap()?;
                            }
                        }
                    } else {
                        println!("unknown header type {} ignoring block", header_type_u8);
                        reader.seek(SeekFrom::Current((block_size - 5) as i64))?;
                    }
                }
                Err(_) => {
                    println!("End of file");
                    break;
                }
            }
        }
        Ok(app)
    }
}

fn with_header_len(
    file: &mut BufWriter<File>,
    header_type: HeaderType,
    f: &dyn Fn(&mut BufWriter<File>) -> std::io::Result<()>,
) -> std::io::Result<()> {
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

fn write_var_field<W: Write>(
    file: &mut W,
    field_id: u32,
    f: &dyn Fn(&mut dyn Write) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let buffer: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(buffer);
    f(&mut cursor)?;
    let written_data = cursor.into_inner();
    write_field_index(file, FieldType::LENGTHDELIMITED, field_id)?;
    leb128::write::unsigned(file, written_data.len() as u64)?;
    file.write_all(&written_data)?;
    Ok(())
}

impl StringIndexer {
    pub fn store(&self, header_type: HeaderType, writer: &mut BufWriter<File>) -> std::io::Result<()> {
        with_header_len(writer, header_type, &|file| {
            let mut compressor = ZlibEncoder::new(file, Compression::default());
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
                    0x00..=0x7F => {}
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
            for (iri, node) in self.iter() {
                write_len_string(iri, file)?;
                let flags: u8 = if node.is_blank_node { 1 } else { 0 } | if node.has_subject { 2 } else { 0 };
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
            let is_blank_node = (flags & 1) > 0;
            let has_subject = (flags & 2) > 0;
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
                properties.push((predicate_index, literal));
            }
            let references_len = leb128::read::unsigned(reader)?;
            let mut references: Vec<(IriIndex, IriIndex)> = Vec::with_capacity(types_len as usize);
            for _ in 0..references_len {
                let predicate_index = leb128::read::unsigned(reader)? as IriIndex;
                let iri_index = leb128::read::unsigned(reader)? as IriIndex;
                references.push((predicate_index, iri_index));
            }
            let reverse_references_len = leb128::read::unsigned(reader)?;
            let mut reverse_references: Vec<(IriIndex, IriIndex)> = Vec::with_capacity(types_len as usize);
            for _ in 0..reverse_references_len {
                let predicate_index = leb128::read::unsigned(reader)? as IriIndex;
                let iri_index = leb128::read::unsigned(reader)? as IriIndex;
                reverse_references.push((predicate_index, iri_index));
            }
            let node = NObject {
                types,
                properties,
                references,
                reverse_references,
                is_blank_node,
                has_subject,
            };
            cache.cache.insert(iri.into(), node);
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
    let field_encoded = (field_index << 3) as u64 | field_type as u64;
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
        FieldType::FLAG => {}
    }
    Ok(())
}

impl IndexSpan {
    pub fn store(&self, writer: &mut BufWriter<File>) -> std::io::Result<()> {
        leb128::write::unsigned(writer, self.start as u64)?;
        leb128::write::unsigned(writer, self.len as u64)?;
        Ok(())
    }
    pub fn restore<R: Read>(reader: &mut R) -> Result<Self> {
        let start = leb128::read::unsigned(reader)? as u32;
        let len = leb128::read::unsigned(reader)? as u32;
        Ok(IndexSpan { start, len })
    }
}

impl Literal {
    pub fn store(&self, writer: &mut BufWriter<File>) -> std::io::Result<()> {
        match self {
            Literal::String(span) => {
                writer.write_u8(1)?;
                span.store(writer)?;
            }
            Literal::LangString(lang_index, span) => {
                writer.write_u8(2)?;
                leb128::write::unsigned(writer, *lang_index as u64)?;
                span.store(writer)?;
            }
            Literal::TypedString(type_index, span) => {
                writer.write_u8(3)?;
                leb128::write::unsigned(writer, *type_index as u64)?;
                span.store(writer)?;
            }
            Literal::StringShort(index) => {
                writer.write_u8(4)?;
                leb128::write::unsigned(writer, *index as u64)?;
            }
        }
        Ok(())
    }
    pub fn restore<R: Read>(reader: &mut R) -> Result<Self> {
        let literal_type = reader.read_u8()?;
        match literal_type {
            1 => {
                let span = IndexSpan::restore(reader)?;
                Ok(Literal::String(span))
            }
            2 => {
                let lang_index = leb128::read::unsigned(reader)? as LangIndex;
                let span = IndexSpan::restore(reader)?;
                Ok(Literal::LangString(lang_index, span))
            }
            3 => {
                let data_type_index = leb128::read::unsigned(reader)? as DataTypeIndex;
                let span = IndexSpan::restore(reader)?;
                Ok(Literal::TypedString(data_type_index, span))
            }
            4 => {
                let index = leb128::read::unsigned(reader)? as IriIndex;
                Ok(Literal::StringShort(index))
            }
            _ => Err(anyhow::anyhow!("Unknown literal type {}", literal_type)),
        }
    }
}

impl PrefixManager {
    pub fn store(&self, writer: &mut BufWriter<File>) -> std::io::Result<()> {
        let len = self.prefixes.len();
        with_header_len(writer, HeaderType::Prefixes, &|file| {
            file.write_u32::<LittleEndian>(len as u32)?;
            {
                let mut compressor = ZlibEncoder::new(file, Compression::default());
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
        let limited_reader = reader.by_ref().take((size - 4) as u64);
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
            if let Ok(nodes) = self.nodes.read() {
                leb128::write::unsigned(writer, nodes.len() as u64)?;
                if let Ok(positions) = self.positions.read() {
                    for (node_layout, node_pos) in nodes.iter().zip(positions.iter()) {
                        leb128::write::unsigned(writer, node_layout.node_index as u64)?;
                        writer.write_f32::<LittleEndian>(node_pos.pos.x)?;
                        writer.write_f32::<LittleEndian>(node_pos.pos.y)?;
                        // Write number of fields
                        leb128::write::unsigned(writer, 0)?;
                    }
                }
            }
            Ok(())
        })
    }

    pub fn restore(reader: &mut BufReader<&File>, _size: u32) -> Result<Self> {
        let len = leb128::read::unsigned(reader)?;
        let mut nodes = Vec::with_capacity(len as usize);
        let mut node_shapes = Vec::with_capacity(len as usize);
        let mut positions = Vec::with_capacity(len as usize);
        let edges = Vec::new();
        for _ in 0..len {
            let node_index = leb128::read::unsigned(reader)? as IriIndex;
            let x = reader.read_f32::<LittleEndian>()?;
            let y = reader.read_f32::<LittleEndian>()?;
            let field_number = leb128::read::unsigned(reader)?;
            for _ in 0..field_number {
                let (field_type, _field_index) = read_field_index(reader)?;
                skip_field(reader, field_type)?;
            }
            nodes.push(NodeLayout { node_index });
            node_shapes.push(NodeShapeData::default());
            positions.push(NodePosition {
                pos: Pos2::new(x, y),
                vel: Vec2::ZERO,
            });
        }
        Ok(SortedNodeLayout {
            nodes: Arc::new(RwLock::new(nodes)),
            positions: Arc::new(RwLock::new(positions)),
            edges: Arc::new(RwLock::new(edges)),
            ..SortedNodeLayout::default()
        })
    }
}

impl GVisualisationStyle {
    pub fn store(&self, writer: &mut BufWriter<File>) -> std::io::Result<()> {
        with_header_len(writer, HeaderType::VisualStyles, &|writer| {
            leb128::write::unsigned(writer, self.node_styles.len() as u64)?;
            for (type_index, style) in self.node_styles.iter() {
                leb128::write::unsigned(writer, *type_index as u64)?;
                leb128::write::unsigned(writer, style.label_index as u64)?;
                leb128::write::unsigned(writer, style.priority as u64)?;
                leb128::write::unsigned(writer, style.max_lines as u64)?;
                let col = style.color.to_array();
                writer.write(&col)?;
                let col_border = style.border_color.to_array();
                writer.write(&col_border)?;
                let col_label = style.label_color.to_array();
                writer.write(&col_label)?;
                writer.write_u8(style.node_shape as u8)?;
                writer.write_u8(style.label_position as u8)?;
                writer.write_u8(style.node_size as u8)?;
                writer.write_f32::<LittleEndian>(style.width)?;
                writer.write_f32::<LittleEndian>(style.height)?;
                writer.write_f32::<LittleEndian>(style.border_width)?;
                writer.write_f32::<LittleEndian>(style.font_size)?;
                writer.write_f32::<LittleEndian>(style.corner_radius)?;
                writer.write_f32::<LittleEndian>(style.label_max_width)?;
                if let Some(icon_style) = &style.icon_style {
                    leb128::write::unsigned(writer, 1)?;
                    write_var_field(writer, 1, &|file| {
                        icon_style.store(file)?;
                        Ok(())
                    })?;
                } else {
                    leb128::write::unsigned(writer, 0)?;
                }
            }
            leb128::write::unsigned(writer, self.edge_styles.len() as u64)?;
            for (reference_index, style) in self.edge_styles.iter() {
                leb128::write::unsigned(writer, *reference_index as u64)?;
                let col = style.color.to_array();
                writer.write(&col)?;
                writer.write_f32::<LittleEndian>(style.width)?;
                writer.write_f32::<LittleEndian>(style.line_gap)?;
                writer.write_f32::<LittleEndian>(style.arrow_size)?;
                writer.write_u8(style.arrow_location as u8)?;
                writer.write_u8(style.line_style as u8)?;
                writer.write_u8(style.target_style as u8)?;
                let mut field_count = 0;
                if style.icon_style.is_some() {
                    field_count += 1;
                }
                if style.edge_font.is_some() {
                    field_count += 1;
                }
                leb128::write::unsigned(writer, field_count)?;
                if let Some(icon_style) = &style.icon_style {
                    write_var_field(writer, 1, &|file| {
                        icon_style.store(file)?;
                        Ok(())
                    })?;
                }
                if let Some(edge_font) = &style.edge_font {
                    write_var_field(writer, 2, &|file| {
                        edge_font.store(file)?;
                        Ok(())
                    })?;
                }
            }
            Ok(())
        })
    }

    pub fn restore(reader: &mut BufReader<&File>, _size: u32) -> Result<Self> {
        let mut styles = GVisualisationStyle {
            node_styles: HashMap::new(),
            edge_styles: HashMap::new(),
            default_node_style: crate::NodeStyle::default(),
        };
        let len_types = leb128::read::unsigned(reader)?;
        for _ in 0..len_types {
            let type_index = leb128::read::unsigned(reader)? as IriIndex;
            let label_index = leb128::read::unsigned(reader)? as IriIndex;
            let priority = leb128::read::unsigned(reader)? as u32;
            let max_lines = leb128::read::unsigned(reader)? as u16;
            let mut color = [0u8; 4];
            reader.read_exact(&mut color)?;
            let mut color_border = [0u8; 4];
            reader.read_exact(&mut color_border)?;
            let mut color_label = [0u8; 4];
            reader.read_exact(&mut color_label)?;
            let node_shape = reader.read_u8()?;
            let node_shape: NodeShape = node_shape
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid node shape value"))?;
            let label_position = reader.read_u8()?;
            let label_position: LabelPosition = label_position
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid label position value"))?;
            let node_size = reader.read_u8()?;
            let node_size: NodeSize = node_size
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid label size value"))?;
            let width = reader.read_f32::<LittleEndian>()?;
            let height = reader.read_f32::<LittleEndian>()?;
            let border_width = reader.read_f32::<LittleEndian>()?;
            let font_size = reader.read_f32::<LittleEndian>()?;
            let corner_radius = reader.read_f32::<LittleEndian>()?;
            let label_max_width = reader.read_f32::<LittleEndian>()?;
            let field_number = leb128::read::unsigned(reader)?;
            let mut icon_style: Option<IconStyle> = None;
            for _ in 0..field_number {
                let (field_type, field_index) = read_field_index(reader)?;
                match field_index {
                    1 => {
                        if field_type == FieldType::LENGTHDELIMITED {
                            icon_style = Some(IconStyle::restore(reader, field_index)?);
                        } else {
                            skip_field(reader, field_type)?;
                        }
                    }
                    _ => {
                        skip_field(reader, field_type)?;
                    }
                }
            }
            let style = crate::NodeStyle {
                color: egui::Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]),
                border_color: egui::Color32::from_rgba_premultiplied(
                    color_border[0],
                    color_border[1],
                    color_border[2],
                    color_border[3],
                ),
                label_color: egui::Color32::from_rgba_premultiplied(
                    color_label[0],
                    color_label[1],
                    color_label[2],
                    color_label[3],
                ),
                priority,
                label_index,
                max_lines,
                width,
                height,
                border_width,
                font_size,
                corner_radius,
                label_max_width,
                node_shape,
                label_position,
                node_size,
                icon_style,
                ..Default::default()
            };
            styles.node_styles.insert(type_index, style);
        }
        let len_references = leb128::read::unsigned(reader)?;
        for _ in 0..len_references {
            let reference_index = leb128::read::unsigned(reader)? as IriIndex;
            let mut color = [0u8; 4];
            reader.read_exact(&mut color)?;
            let width = reader.read_f32::<LittleEndian>()?;
            let line_gap = reader.read_f32::<LittleEndian>()?;
            let arrow_size = reader.read_f32::<LittleEndian>()?;
            let arrow_location = reader.read_u8()?;
            let arrow_location: ArrowLocation = arrow_location
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid arrow_location value"))?;
            let line_style = reader.read_u8()?;
            let line_style: LineStyle = line_style
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid line_style value"))?;
            let target_style = reader.read_u8()?;
            let target_style: ArrowStyle = target_style
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid target_style value"))?;
            let mut icon_style: Option<IconStyle> = None;
            let mut edge_font: Option<EdgeFont> = None;

            let field_number = leb128::read::unsigned(reader)?;
            for _ in 0..field_number {
                let (field_type, field_index) = read_field_index(reader)?;
                match field_index {
                    1 => {
                        if field_type == FieldType::LENGTHDELIMITED {
                            icon_style = Some(IconStyle::restore(reader, field_index)?);
                        } else {
                            skip_field(reader, field_type)?;
                        }
                    }
                    2 => {
                        if field_type == FieldType::LENGTHDELIMITED {
                            edge_font = Some(EdgeFont::restore(reader, field_index)?);
                        } else {
                            skip_field(reader, field_type)?;
                        }
                    }
                    _ => {
                        skip_field(reader, field_type)?;
                    }
                }
            }
            let style = EdgeStyle {
                color: egui::Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]),
                width,
                line_gap,
                arrow_size,
                line_style,
                arrow_location,
                target_style,
                icon_style,
                edge_font,
            };

            styles.edge_styles.insert(reference_index, style);
        }

        Ok(styles)
    }
}

impl IconStyle {
    pub fn store<W: Write + ?Sized>(&self, writer: &mut W) -> std::io::Result<()> {
        leb128::write::unsigned(writer, self.icon_position as u64)?;
        writer.write_f32::<LittleEndian>(self.icon_size)?;
        let col = self.icon_color.to_array();
        writer.write(&col)?;
        let character = self.icon_character as u32;
        leb128::write::unsigned(writer, character as u64)?;
        leb128::write::unsigned(writer, 0)?;
        Ok(())
    }

    pub fn restore(reader: &mut BufReader<&File>, _size: u32) -> Result<Self> {
        let _field_length = leb128::read::unsigned(reader)?;
        let mut icon_style = IconStyle::default();
        let icon_position = reader.read_u8()?;
        icon_style.icon_position = icon_position
            .try_into()
            .map_err(|_| anyhow::anyhow!("Invalid icon shape value"))?;
        icon_style.icon_size = reader.read_f32::<LittleEndian>()?;
        let mut color = [0u8; 4];
        reader.read_exact(&mut color)?;
        icon_style.icon_color = egui::Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]);
        let character = leb128::read::unsigned(reader)? as u32;
        icon_style.icon_character =
            char::from_u32(character).ok_or_else(|| anyhow::anyhow!("Invalid icon character value"))?;
        let field_number = leb128::read::unsigned(reader)?;
        for _ in 0..field_number {
            let (field_type, _field_index) = read_field_index(reader)?;
            skip_field(reader, field_type)?;
        }
        Ok(icon_style)
    }
}

impl EdgeFont {
    pub fn store<W: Write + ?Sized>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_f32::<LittleEndian>(self.font_size)?;
        let col = self.font_color.to_array();
        writer.write(&col)?;
        // num of fields
        leb128::write::unsigned(writer, 0)?;
        Ok(())
    }

    pub fn restore(reader: &mut BufReader<&File>, _size: u32) -> Result<Self> {
        let _field_length = leb128::read::unsigned(reader)?;
        let mut edge_font = EdgeFont::default();
        edge_font.font_size = reader.read_f32::<LittleEndian>()?;
        let mut color = [0u8; 4];
        reader.read_exact(&mut color)?;
        edge_font.font_color = egui::Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3]);
        let field_number = leb128::read::unsigned(reader)?;
        for _ in 0..field_number {
            let (field_type, _field_index) = read_field_index(reader)?;
            skip_field(reader, field_type)?;
        }
        Ok(edge_font)
    }
}

impl StringCache {
    pub fn store(&self, writer: &mut BufWriter<File>) -> std::io::Result<()> {
        with_header_len(writer, HeaderType::Literals, &|file| {
            let mut compressor = ZlibEncoder::new(file, Compression::default());
            compressor.write_all(self.cache.as_bytes())?;
            compressor.finish()?;
            Ok(())
        })
    }

    pub fn restore<R: Read>(reader: &mut R, size: u32) -> Result<Self> {
        let limited_reader = reader.by_ref().take(size as u64);
        let mut decoder = ZlibDecoder::new(limited_reader);
        let mut str = String::new();
        decoder.read_to_string(&mut str)?;
        let index = StringCache { cache: str };
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::Instant};

    use egui::Color32;

    use crate::{graph_styles::{ArrowLocation, IconPosition, IconStyle, LineStyle}, NodeChangeContext};

    use super::*;

    fn get_test_file_path(filename: &str) -> PathBuf {
        let mut dir = PathBuf::from("target/test-files");
        fs::create_dir_all(&dir).expect("Failed to create test directory"); // Ensure directory exists
        dir.push(filename);
        dir
    }

    #[test]
    fn test_store() -> std::io::Result<()> {
        let store_path = get_test_file_path("store.rdfglance");

        let mut vs = RdfGlanceApp::new(None);
        let start = Instant::now();
        vs.load_ttl("sample-rdf-data/programming_languages.ttl");
        vs.join_load();

        if let Ok(rdf_data) = vs.rdf_data.read() {
            println!("nodes read {}", rdf_data.node_data.len());
            let duration = start.elapsed();
            println!("Time taken to read ttl {:?}", duration);
        }
        if let Ok(mut rdf_data) = vs.rdf_data.write() {
            let label_index = rdf_data.node_data.indexers.predicate_indexer.get_index("rdfs:label");
            assert_eq!(0, label_index);
        }
        let node_index = vs
            .rdf_data
            .read()
            .unwrap()
            .node_data
            .get_node_index("dbr:Rust_(programming_language)");
        assert_eq!(true, node_index.is_some());
        if let Ok(mut rdf_data) = vs.rdf_data.write() {
            let mut node_change_context =  NodeChangeContext {
                rdfwrwap: &mut vs.rdfwrap,
                visible_nodes: &mut vs.visible_nodes,
            };
            assert_eq!(true, rdf_data.load_object_by_index(node_index.unwrap(),&mut node_change_context));
        };
        if let Ok(mut rdf_data) = vs.rdf_data.write() {
            let mut node_change_context =  NodeChangeContext {
                rdfwrwap: &mut vs.rdfwrap,
                visible_nodes: &mut vs.visible_nodes,
            };
            rdf_data.expand_all(&mut node_change_context);
        }
        assert_eq!(true, vs.visible_nodes.nodes.read().unwrap().len() > 0);
        if let Ok(rdf_data) = vs.rdf_data.read() {
            let (node_iri, node_object) = rdf_data.node_data.get_node_by_index(node_index.unwrap()).unwrap();
            assert_eq!("dbr:Rust_(programming_language)", node_iri.to_string());
            assert_eq!(1, node_object.types.len());
            let type_index = node_object.types.get(0).unwrap();
            let type_style = vs.visualisation_style.node_styles.get_mut(type_index).unwrap();
            type_style.max_lines = 2;
            type_style.node_shape = NodeShape::Rect;
            type_style.label_position = LabelPosition::Above;
            type_style.node_size = NodeSize::Label;
            type_style.width = 100.0;
            type_style.height = 50.0;
            type_style.corner_radius = 4.0;
            type_style.label_max_width = 80.0;
            type_style.border_width = 4.0;
            type_style.border_color = Color32::YELLOW;
            type_style.color = Color32::RED;
            type_style.label_color = Color32::GRAY;
            type_style.icon_style = Some({
                IconStyle {
                    icon_color: Color32::GRAY,
                    icon_character: 'R',
                    icon_size: 20.0,
                    icon_position: IconPosition::Above,
                }
            });
            let edge_index = node_object.references.get(0).unwrap().0;
            vs.visualisation_style.get_edge_syle(edge_index);
            let edge = vs.visualisation_style.edge_styles.get_mut(&edge_index).unwrap();
            edge.color = Color32::YELLOW;
            edge.width = 3.0;
            edge.line_style = LineStyle::Dashed;
            edge.arrow_location = ArrowLocation::Middle;
            edge.arrow_size = 10.0;
            edge.line_gap = 4.0;
            edge.edge_font = Some(EdgeFont {
                font_color: Color32::GRAY,
                font_size: 20.0,
            });
            edge.icon_style = Some({
                IconStyle {
                    icon_color: Color32::GRAY,
                    icon_character: 'R',
                    icon_size: 20.0,
                    icon_position: IconPosition::Above,
                }
            });
        }

        vs.store(&store_path)?;

        assert!(store_path.exists(), "file does not exists");
        let start = Instant::now();
        let mut restored = RdfGlanceApp::restore(&store_path).unwrap();
        let duration = start.elapsed();
        println!("Time taken to read project {:?}", duration);

        restored.read_rdf_data(|restored_rdf_data| {
            vs.read_rdf_data(|rdf_data| {
                assert_eq!(
                    rdf_data.node_data.indexers.datatype_indexer.map.len(),
                    restored_rdf_data.node_data.indexers.datatype_indexer.map.len()
                );
                assert_eq!(
                    rdf_data.node_data.indexers.language_indexer.map.len(),
                    restored_rdf_data.node_data.indexers.language_indexer.map.len()
                );
                assert_eq!(
                    rdf_data.node_data.indexers.predicate_indexer.map.len(),
                    restored_rdf_data.node_data.indexers.predicate_indexer.map.len()
                );
                assert_eq!(
                    rdf_data.node_data.indexers.type_indexer.map.len(),
                    restored_rdf_data.node_data.indexers.type_indexer.map.len()
                );
            });
        });

        if let Ok(restored_rdf_data) = restored.rdf_data.read() {
            let rust_node = restored_rdf_data.node_data.get_node("dbr:Rust_(programming_language)");
            assert!(rust_node.is_some(), "rust node not found");
            if let Some(rust_node) = rust_node {
                rust_node.references.iter().for_each(|(pred, ref_index)| {
                    let pred_str = restored_rdf_data
                        .node_data
                        .indexers
                        .predicate_indexer
                        .index_to_str(*pred)
                        .unwrap();
                    assert!(pred_str.len() > 1);
                    restored_rdf_data.node_data.get_node_by_index(*ref_index);
                });
                rust_node.properties.iter().for_each(|(pred, literal)| {
                    let pred_str = restored_rdf_data
                        .node_data
                        .indexers
                        .predicate_indexer
                        .index_to_str(*pred)
                        .unwrap();
                    assert!(pred_str.len() > 1);
                    let lit_str = literal.as_str_ref(&restored_rdf_data.node_data.indexers);
                    assert!(lit_str.len() > 1);
                });
                assert_eq!(1, rust_node.types.len());
                let type_index = rust_node.types.get(0).unwrap();
                let type_style = restored.visualisation_style.node_styles.get_mut(type_index).unwrap();
                assert_eq!(type_style.max_lines, 2);
                assert_eq!(type_style.node_shape, NodeShape::Rect);
                assert_eq!(type_style.label_position, LabelPosition::Above);
                assert_eq!(type_style.node_size, NodeSize::Label);
                assert_eq!(type_style.width, 100.0);
                assert_eq!(type_style.height, 50.0);
                assert_eq!(type_style.corner_radius, 4.0);
                assert_eq!(type_style.label_max_width, 80.0);
                assert_eq!(type_style.border_width, 4.0);
                assert_eq!(type_style.border_color, Color32::YELLOW);
                assert_eq!(type_style.color, Color32::RED);
                assert_eq!(type_style.label_color, Color32::GRAY);
                assert_eq!(type_style.icon_style.is_some(), true);
                if let Some(icon_style) = type_style.icon_style.as_ref() {
                    assert_eq!(icon_style.icon_color, Color32::GRAY);
                    assert_eq!(icon_style.icon_character, 'R');
                    assert_eq!(icon_style.icon_size, 20.0);
                    assert_eq!(icon_style.icon_position, IconPosition::Above);
                } else {
                    panic!("Icon style not found");
                }
            }
            /*
            let edge = restored.visualisation_style.edge_styles.get_mut(&edge_index).unwrap();
            assert_eq!(edge.color, Color32::YELLOW);
            assert_eq!(edge.width, 3.0);
            assert_eq!(edge.line_style, LineStyle::Dashed);
            assert_eq!(edge.arrow_location, ArrowLocation::Middle);
            assert_eq!(edge.arrow_size, 10.0);
            assert_eq!(edge.line_gap, 4.0);
            assert_eq!(edge.icon_style.is_some(), true);
            if let Some(icon_style) = &edge.icon_style {
                assert_eq!(icon_style.icon_color, Color32::GRAY);
                assert_eq!(icon_style.icon_character, 'R');
                assert_eq!(icon_style.icon_size, 20.0);
                assert_eq!(icon_style.icon_position, IconPosition::Above);
            } else {
                panic!("Icon style not found");
            }
            assert_eq!(edge.edge_font.is_some(), true);
            if let Some(edge_font) = &edge.edge_font {
                assert_eq!(edge_font.font_color, Color32::GRAY);
                assert_eq!(edge_font.font_size, 20.0);
            } else {
                panic!("Edge font not found");
            }
            */
        }
        let predicates = vec!["rdf:type"];
        for pred_val in &predicates {
            assert!(
                vs.rdf_data
                    .write()
                    .unwrap()
                    .node_data
                    .indexers
                    .get_predicate_index(pred_val)
                    == restored
                        .rdf_data
                        .write()
                        .unwrap()
                        .node_data
                        .indexers
                        .get_predicate_index(pred_val)
            )
        }
        assert_eq!(
            vs.rdf_data.read().unwrap().node_data.len(),
            restored.rdf_data.read().unwrap().node_data.len()
        );
        assert_eq!(
            vs.rdf_data.read().unwrap().prefix_manager.prefixes.len(),
            restored.rdf_data.read().unwrap().prefix_manager.prefixes.len()
        );

        assert_eq!(
            vs.visible_nodes.nodes.read().unwrap().len(),
            restored.visible_nodes.nodes.read().unwrap().len()
        );
        assert_eq!(
            vs.visualisation_style.node_styles.len(),
            restored.visualisation_style.node_styles.len()
        );
        assert_eq!(true, vs.visible_nodes.nodes.read().unwrap().len() > 0);

        Ok(())
    }
}
