use std::fs::File;
use std::io::{Write, Read, BufWriter, BufReader};
use std::path::Path;
use byteorder::{WriteBytesExt, ReadBytesExt, LittleEndian};

use crate::VisualRdfApp;

const MAGIC_NUMBER: u32 = 0x52444647;
const FORMAT_VERSION: u16 = 0;
const FORMAT_FLAGS: u16 = 0;

impl VisualRdfApp {
    
    pub fn store(&self, path: &Path)  -> std::io::Result<()> {
        let mut file = BufWriter::new(File::create(path)?);

        let number: u32 = 42;
        file.write_u32::<LittleEndian>(MAGIC_NUMBER)?;
        file.write_u16::<LittleEndian>(FORMAT_VERSION)?;
        file.write_u16::<LittleEndian>(FORMAT_FLAGS)?;
        // header size 
        file.write_u16::<LittleEndian>(10)?;



        file.flush()?;
        Ok(())
    }

    pub fn restore(path: &Path) -> Self {
        let mut app = VisualRdfApp::new(None);
        return app;
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
        
        let mut vs = VisualRdfApp::new(None);
        vs.load_ttl("sample-rdf-data/programming_languages.ttl");
        vs.store(&store_path)?;

        assert!(store_path.exists(),"file does not exists");

        Ok(())
    }
   
}

