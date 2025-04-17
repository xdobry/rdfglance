use string_interner::{backend::StringBackend, symbol::SymbolU32, StringInterner, Symbol};

use crate::nobject::IriIndex;

pub struct StringIndexer {
    pub map: StringInterner<StringBackend>,
}

impl Default for StringIndexer {
    fn default() -> Self {
        Self::new()
    }
}

impl StringIndexer {
    pub fn new() -> Self {
        Self { map: StringInterner::default() }
    }

    /// Converts a string to an index, assigning a new index if it's unknown
    pub fn get_index(&mut self, s: &str) -> IriIndex {
        let index = self.map.get_or_intern(s);
        index.to_usize() as IriIndex
    }

    /// Retrieves a string from an index
    pub fn index_to_str(&self, index: IriIndex) -> Option<&str> {
        self.map.resolve(SymbolU32::try_from_usize(index as usize).unwrap())
    }
    
}

pub struct StringCache {
    pub cache: String,
}


/// A 'StrignCache' is a simple string cache that allows for storing and retrieving strings by their index.
/// It is used to store strings in a single contiguous memory block, allowing for efficient access and storage.
impl StringCache {
    pub fn new() -> Self {
        Self { cache: String::new() }
    }

    pub fn push_str(&mut self, s: &str) -> IndexSpan {
        let end_pos = self.cache.len() as u32;
        let length = s.len() as u32;
        self.cache.push_str(s);
        IndexSpan { start: end_pos, len: length }
    }

    pub fn get_str(&self, span: IndexSpan) -> &str {
        let start = span.start as usize;
        let end = (span.start + span.len) as usize;
        &self.cache[start..end]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexSpan {
    pub start: u32,
    pub len: u32,
}

#[cfg(test)]
mod tests {
    use super::StringIndexer;

    #[test]
    fn test_sting_indexer() {
        let mut string_indexer = StringIndexer::new();
        let index1 = string_indexer.get_index("test");
        let index2 = string_indexer.get_index("test");
        assert_eq!(0, index1);
        assert_eq!(index1, index2);
        let index3 = string_indexer.get_index("test2");
        assert_ne!(index1, index3);
        assert_eq!(index1+1, index3);
        let s = string_indexer.index_to_str(index2);
        assert!(s.is_some());
        assert_eq!("test",s.unwrap());
        assert!(string_indexer.index_to_str(100).is_none());  
    }

    #[test]
    fn test_string_cache() {
        let mut string_cache = super::StringCache::new();
        let span1 = string_cache.push_str("test");
        let span2 = string_cache.push_str("test2");
        assert_eq!(span1.start, 0);
        assert_eq!(span1.len, 4);
        assert_eq!(span2.start, 4);
        assert_eq!(span2.len, 5);
        assert_eq!("test", string_cache.get_str(span1));
        assert_eq!("test2", string_cache.get_str(span2));
    }
}