use std::{ffi::OsStr, fmt::Display, io, path::PathBuf};

use futures_lite::{future::block_on, StreamExt};
use pyo3::{PyErr, Python};
use walkdir::WalkDir;

fn unicode_decode_err(os_string: &OsStr) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Unable to convert path to unicode: {}",
            os_string.to_string_lossy()
        ),
    )
}

pub enum IterdirErr {
    Io(io::Error),
    Interrupt(PyErr),
}

impl Display for IterdirErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Interrupt(err) => f.write_fmt(format_args!("{}", err)),
            Self::Io(err) => f.write_fmt(format_args!("{}", err)),
        }
    }
}

pub fn iterdir<F: FnMut(String)>(path: PathBuf, mut callback: F) -> Result<(), IterdirErr> {
    Python::with_gil(|py| {
        if path.is_file() {
            return Ok(callback(
                path.into_os_string()
                    .into_string()
                    .map_err(|e| IterdirErr::Io(unicode_decode_err(&e)))?,
            ));
        } else if path.exists() {
            WalkDir::new(&path)
                .into_iter()
                .filter_entry(|entry| {
                    if let Some(true) = entry.path().file_name().map(|f| {
                        let path = f.to_string_lossy();
                        path == "dataset_description.json"
                            || path.starts_with(".")
                            || path == "derivatives"
                    }) {
                        false
                    } else if let Some(true) = entry
                        .path()
                        .file_name()
                        .map(|f| f.to_string_lossy().starts_with('.'))
                    {
                        false
                    } else {
                        true
                    }
                })
                .map(|entry| match entry {
                    Ok(entry) => {
                        if !entry.path().is_dir() {
                            match entry.path().to_str() {
                                Some(path) => callback(path.to_string()),
                                None => {
                                    return Err(IterdirErr::Io(unicode_decode_err(
                                        entry.path().as_os_str(),
                                    )))
                                }
                            };
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
