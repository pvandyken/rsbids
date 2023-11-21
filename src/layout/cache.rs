use std::{
    fs,
    io::{self, Read, Write},
    path::PathBuf,
};

use crate::errors::CacheErr;

use super::Layout;

const DECLARATION: &[u8] = "<?rsbids version=\"1.0\">\n".as_bytes();

pub struct LayoutCache;

impl LayoutCache {
    fn write(path: PathBuf, data: Vec<u8>) -> io::Result<()> {
        let decleration = Vec::from(DECLARATION);
        let mut file = fs::File::create(path)?;
        file.write_all(&decleration)?;
        file.write_all(&data)?;
        Ok(())
    }

    fn read(path: PathBuf) -> io::Result<Vec<u8>> {
        let mut declaration = [0u8; DECLARATION.len()];
        let mut file = fs::File::open(path.clone())?;
        file.read_exact(&mut declaration)?;
        if declaration == DECLARATION {
            let mut encoded: Vec<u8> = Vec::new();
            file.read_to_end(&mut encoded)?;
            Ok(encoded)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("'{}' is not a valid rsbids cache file", path.to_string_lossy()),
            ))
        }
    }

    pub fn save(layout: &Layout, path: PathBuf) -> Result<(), CacheErr> {
        let encoded = bincode::serialize(layout)?;
        Self::write(path, encoded).map_err(|err| Box::new(bincode::ErrorKind::Io(err)))?;
        Ok(())
    }

    pub fn load(path: PathBuf) -> Result<Layout, CacheErr> {
        let encoded = Self::read(path).map_err(|err| Box::new(bincode::ErrorKind::Io(err)))?;
        Ok(bincode::deserialize(&encoded)?)
    }
}
