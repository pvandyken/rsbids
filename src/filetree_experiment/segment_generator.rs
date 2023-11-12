use std::{
    ffi::OsString,
    io,
    iter::once,
    path::{Component, PathBuf},
};

use itertools::Itertools;
use walkdir::WalkDir;

pub enum SegmentType {
    File(String),
    Directory(String),
}

pub enum ReversePath {
    File(Vec<String>),
    Directory(Vec<String>),
}

pub enum NewSegment {
    Ordered(ReversePath),
    NonOrdered(ReversePath),
}
pub struct SegmentGenerator {
    deck: Vec<String>,
    is_dir: bool,
    ordered: bool,
    next_segments: Box<dyn Iterator<Item = NewSegment>>,
}

enum UnparsedPath {
    File(String),
    Directory(String),
}

fn unicode_decode_err(os_string: OsString) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Unable to convert path to unicode: {}",
            os_string.to_string_lossy()
        ),
    )
}

fn to_components(path: UnparsedPath) -> ReversePath {
    let (path, is_dir) = match path {
        UnparsedPath::Directory(path) => (path, true),
        UnparsedPath::File(path) => (path, false),
    };
    let segments = PathBuf::from(path)
        .components()
        .filter_map(|component| match component {
            Component::Normal(comp) => Some(comp.to_string_lossy().into_owned()),
            _ => None,
        })
        .rev()
        .collect_vec();
    if is_dir {
        ReversePath::Directory(segments)
    } else {
        ReversePath::File(segments)
    }
}

impl SegmentGenerator {
    pub fn new<'a>(paths: Vec<String>) -> SegmentGenerator {
        let segment_generator =
            paths
                .into_iter()
                .map(
                    |path| -> Box<dyn Iterator<Item = Result<UnparsedPath, io::Error>>> {
                        let path = PathBuf::from(path);
                        if path.is_file() {
                            Box::new(once(
                                path.into_os_string()
                                    .into_string()
                                    .map_err(|e| unicode_decode_err(e))
                                    .map(|path| UnparsedPath::File(path)),
                            ))
                        } else {
                            Box::new(
                                WalkDir::new(path)
                                    .into_iter()
                                    .filter_entry(|entry| {
                                        // dbg!(&entry);
                                        if let Some(true) = entry.path().file_name().map(|f| {
                                            f.to_string_lossy() == "dataset_description.json"
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
                                    .map(|entry| {
                                        // dbg!(&entry);
                                        match entry {
                                            Ok(entry) => {
                                                let is_dir = entry.path().is_dir();
                                                match entry.path().to_str() {
                                                    Some(path) => {
                                                        if is_dir {
                                                            Ok(UnparsedPath::Directory(
                                                                path.to_string(),
                                                            ))
                                                        } else {
                                                            Ok(UnparsedPath::File(path.to_string()))
                                                        }
                                                    }
                                                    None => {
                                                        return Err(io::Error::new(
                                                            io::ErrorKind::InvalidData,
                                                            "Error",
                                                        ))
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                return Err(io::Error::new(
                                                    io::ErrorKind::InvalidData,
                                                    e.to_string(),
                                                ))
                                            }
                                        }
                                    }),
                            )
                        }
                    },
                )
                .map(|paths| {
                    let mut got_first = false;
                    paths.map(move |path| {
                        path.map(|path| {
                            if got_first {
                                NewSegment::Ordered(to_components(path))
                            } else {
                                got_first = true;
                                NewSegment::NonOrdered(to_components(path))
                            }
                        })
                    })
                })
                .flatten()
                .filter_map(|path| match path {
                    Ok(path) => Some(path),
                    Err(_) => None,
                });
        SegmentGenerator {
            deck: Vec::new(),
            is_dir: false,
            ordered: false,
            next_segments: Box::new(segment_generator),
        }
    }
    pub fn next(&mut self) -> Option<SegmentType> {
        self.deck.pop().map(|seg| {
            if self.deck.len() == 0 && !self.is_dir {
                SegmentType::File(seg)
            } else {
                SegmentType::Directory(seg)
            }
        })
    }

    pub fn ordered(&self) -> bool {
        self.ordered
    }

    pub fn next_segment(&mut self) -> Option<()> {
        self.next_segments
            .next()
            .map(|path| match path {
                NewSegment::NonOrdered(path) => {
                    self.ordered = false;
                    path
                }
                NewSegment::Ordered(path) => {
                    self.ordered = true;
                    path
                }
            })
            .map(|path| match path {
                ReversePath::Directory(path) => {
                    self.deck = path;
                    self.is_dir = true;
                }
                ReversePath::File(path) => {
                    self.deck = path;
                    self.is_dir = false
                }
            })
    }

    pub fn get(&self, i: usize) -> Option<&String> {
        let len = self.deck.len();
        if i >= len {
            None
        } else {
            self.deck.get(self.deck.len() - i - 1)
        }
    }
    // pub fn pull(&mut self, i: usize) -> String {
    //     while self.deck.len() > self.len - i {
    //         self.deck
    //             .pop()
    //             .expect("Should have value given our length check");
    //     }
    //     self.deck
    //         .pop()
    //         .expect("Should have value given our length check")
    // }
}
