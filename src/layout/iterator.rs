use std::{collections::HashSet, ops::Range, sync::Arc};

use super::bidspath::BidsPath;

pub struct BidsPathViewIterator {
    paths: Arc<Vec<BidsPath>>,
    valid_entities: HashSet<String>,
    iterator: Box<dyn Iterator<Item = usize> + Send>,
}

impl<'a> BidsPathViewIterator {
    pub fn new(
        paths: Arc<Vec<BidsPath>>,
        entities: HashSet<String>,
        indices: Option<Vec<usize>>,
    ) -> BidsPathViewIterator {
        let len = paths.len();
        BidsPathViewIterator {
            paths,
            valid_entities: entities,
            iterator: {
                if let Some(indices) = indices {
                    Box::new(indices.into_iter())
                } else {
                    Box::new(Range::from(0..len).into_iter())
                }
            },
        }
    }
}

impl Iterator for BidsPathViewIterator {
    type Item = BidsPath;
    fn next(&mut self) -> Option<Self::Item> {
        let i = self.iterator.next()?;
        let mut path = self.paths.get(i).unwrap().clone();
        path.update_parents(&self.valid_entities);
        Some(path)
    }
}
