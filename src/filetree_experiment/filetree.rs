use std::{
    collections::HashMap,
    ffi::OsString,
    io,
    iter::once,
    path::{Component, PathBuf},
};

use itertools::Itertools;
use walkdir::WalkDir;

use crate::{
    primitives::ComponentType,
    segment_generator::{NewSegment, ReversePath, SegmentGenerator, SegmentType, self},
    segments::{classify_component, group_primitives, parse_path_segment},
};

#[derive(Debug)]
struct FileTreeFile {
    segment: String,
    segment_type: ComponentType,
}

#[derive(Debug)]
struct FileTreeDir {
    node: FileTreeNode,
    level: usize,
    segment: String,
    segment_type: ComponentType,
}

#[derive(Debug)]
struct FileTreeNode {
    children: HashMap<String, FileTreeDir>,
    files: Vec<FileTreeFile>,
}

#[derive(Debug)]
pub struct FileTree {
    root: FileTreeNode,
}



impl FileTree {
    pub fn new(paths: Vec<String>) -> FileTree {
        let mut filetree = FileTree {
            root: FileTreeNode {
                children: HashMap::new(),
                files: Vec::new(),
            },
        };
        let mut segment_generator = SegmentGenerator::new(paths);

        let mut next = segment_generator.next_segment().map(|_| segment_generator);
        loop {
            match next {
                Some(seg_gen) => next = filetree.root.parse(seg_gen, 0),
                None => break,
            }
        }
        filetree
    }
}

impl FileTreeDir {
    fn new(segment: String, level: usize) -> FileTreeDir {
        let seg_type = FileTreeNode::parse_segment(&segment);
        FileTreeDir {
            segment,
            segment_type: seg_type,
            level,
            node: FileTreeNode {
                children: HashMap::new(),
                files: Vec::new(),
            },
        }
    }

    pub fn get_segment_type(&self) -> &ComponentType {
        &self.segment_type
    }

    pub fn parse(&mut self, segments: SegmentGenerator) -> Option<SegmentGenerator> {
        let next = self.node.parse(segments, self.level + 1)?;
        if next.ordered() {
            match next.get(self.level) {
                Some(seg) => {
                    if seg == &self.segment {
                        self.parse(next)
                    } else {
                        Some(next)
                    }
                }
                None => Some(next),
            }
        } else {
            Some(next)
        }
    }
}

impl FileTreeNode {
    pub fn parse(
        &mut self,
        mut segments: SegmentGenerator,
        next_level: usize,
    ) -> Option<SegmentGenerator> {
        match segments.next() {
            Some(segment) => match segment {
                SegmentType::Directory(segment) => match self.children.get_mut(&segment) {
                    Some(child) => child.parse(segments),
                    None => {
                        let mut new_node = FileTreeDir::new(segment, next_level);
                        let segments = new_node.parse(segments);
                        self.children.insert(new_node.segment.clone(), new_node);
                        segments
                    }
                },
                SegmentType::File(segment) => {
                    let seg_type = FileTreeNode::parse_segment(&segment);
                    let file = FileTreeFile {
                        segment,
                        segment_type: seg_type,
                    };
                    self.files.push(file);
                    segments.next_segment().map(|_| segments)
                }
            },
            None => segments.next_segment().map(|_| segments),
        }

        // fn next_segment(&self, mut segments: Box<SegmentGenerator>) ->
    }

    fn parse_segment(segment: &str) -> ComponentType {
        let elements = parse_path_segment(segment);
        let elements = group_primitives(elements);
        classify_component(elements)
    }
}
