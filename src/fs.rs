use std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
    io,
    path::PathBuf,
};

use futures_lite::{future::block_on, StreamExt};
use pyo3::Python;
use walkdir::WalkDir;

use crate::errors::IterdirErr;

fn unicode_decode_err(os_string: &OsStr) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Unable to convert path to unicode: {}",
            os_string.to_string_lossy()
        ),
    )
}

pub struct IterIgnore {
    pub paths: HashSet<PathBuf>,
    pub names: HashSet<OsString>,
}

impl IterIgnore {
    pub fn new() -> Self {
        Self {
            paths: HashSet::new(),
            names: HashSet::new(),
        }
    }
}

pub fn iterdir<F: FnMut(PathBuf)>(
    path: PathBuf,
    ignore: &IterIgnore,
    mut callback: F,
) -> Result<(), IterdirErr> {
    Python::with_gil(|py| {
        if path.is_file() {
            return Ok(callback(path));
        } else if path.exists() {
            WalkDir::new(&path)
                .into_iter()
                .filter_entry(|entry| {
                    if entry.path() == path {
                        true
                    } else if let Some(true) = entry.path().file_name().map(|f| {
                        ignore.names.contains(f)
                            || f.to_str().map(|s| s.starts_with('.')).unwrap_or(false)
                    }) {
                        false
                    } else if ignore.paths.contains(entry.path()) {
                        false
                    } else {
                        true
                    }
                })
                .map(|entry| match entry {
                    Ok(entry) => {
                        if !entry.path().is_dir() {
                            callback(entry.into_path());
                        };
                        Ok(())
                    }
                    Err(e) => {
                        return Err(IterdirErr::Io(io::Error::new(
                            io::ErrorKind::InvalidData,
                            e.to_string(),
                        )))
                    }
                })
                .collect::<Result<Vec<_>, IterdirErr>>()?;
            if let Err(err) = py.check_signals() {
                Err(IterdirErr::Interrupt(err))
            } else {
                Ok(())
            }
            // Ok(())
        } else {
            Err(IterdirErr::Io(io::Error::new(
                io::ErrorKind::NotFound,
                format!("File not found: {}", path.to_string_lossy()),
            )))
        }
    })
}

pub fn iterdir_async<F: FnMut(String)>(path: PathBuf, mut callback: F) -> Result<(), io::Error> {
    if path.is_file() {
        return Ok(callback(
            path.into_os_string()
                .into_string()
                .map_err(|e| unicode_decode_err(&e))?,
        ));
    } else if path.exists() {
        block_on(async {
            let mut entries = async_walkdir::WalkDir::new(path).filter(|entry| async move {
                if entry.path().is_dir() {
                    async_walkdir::Filtering::Ignore
                } else if let Some(true) = entry
                    .path()
                    .file_name()
                    .map(|f| f.to_string_lossy() == "dataset_description.json")
                {
                    async_walkdir::Filtering::Ignore
                } else if let Some(true) = entry
                    .path()
                    .file_name()
                    .map(|f| f.to_string_lossy().starts_with('.'))
                {
                    return async_walkdir::Filtering::IgnoreDir;
                } else {
                    async_walkdir::Filtering::Continue
                }
            });
            loop {
                match entries.next().await {
                    Some(Ok(entry)) => match entry.path().into_os_string().into_string() {
                        Ok(path) => callback(path),
                        Err(e) => return Err(unicode_decode_err(&e)),
                    },
                    Some(Err(e)) => return Err(e),
                    None => break,
                };
            }
            Ok(())
        })
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File not found: {}", path.to_string_lossy()),
        ))
    }
}
