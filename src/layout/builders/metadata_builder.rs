use std::collections::{BTreeMap, HashMap, HashSet};

use itertools::Itertools;

use crate::{
    construct_query,
    errors::MetadataIndexErr,
    layout::{entity_table::EntityTable, Layout},
};

use super::layout_builder::FileTree;

pub type MetadataIndexResult = Result<HashMap<String, String>, MetadataIndexErr>;

#[derive(Default)]
pub struct MetadataIndexBuilder {
    pub metadata: EntityTable<String>,
    was_assigned: HashMap<String, HashSet<usize>>,
}

impl MetadataIndexBuilder {
    pub fn add_entry(&mut self, key: &str, val: &serde_json::Value, ix: &HashSet<usize>) {
        use serde_json::Value;
        let val = match val {
            Value::String(str) => str.as_str(),
            Value::Null => "null",
            Value::Bool(b) => {
                if *b {
                    "true"
                } else {
                    "false"
                }
            }
            Value::Number(x) => return self.add_entry(key, &Value::String(x.to_string()), ix),
            _ => return (),
        };
        if let Some(assign) = self.was_assigned.get_mut(key) {
            let tbd = ix.difference(assign).cloned().collect_vec();
            self.metadata.extend_entities(key, val, tbd.iter().cloned());
            assign.extend(tbd);
        } else {
            self.metadata.extend_entities(key, val, ix.iter().cloned());
            self.was_assigned.insert(key.to_string(), ix.clone());
        }
    }

    pub fn build(
        depths: &BTreeMap<usize, HashSet<usize>>,
        filetree: &FileTree,
        layout: &Layout,
    ) -> MetadataIndexBuilder {
        let mut md_builder = Self::default();
        for vals in depths.values().rev() {
            // Get all json files at depth. If error, nothing was found, so just continue
            if let Ok(sub) = layout.query(construct_query!("extension": ".json"), None, Some(vals))
            {
                for md in sub.get_paths() {
                    // For now, we ignore all errors related to metadata handling
                    // Eventually these can be escalated based on configuration
                    let _ = || -> Result<(), MetadataIndexErr> {
                        if let Some(ixs) = filetree
                            .get_subfiles(&md.as_path().parent().expect("Should have a parent"))
                        {
                            let ref_entities = md.get_full_entities();
                            let ixs = ixs
                                .into_iter()
                                .filter(|ix| {
                                    let child_path = layout.get_path(*ix).expect(
                                        "Internal state of filetree should match that of layout",
                                    );
                                    let path_entities = child_path.get_full_entities();
                                    for (key, val) in &ref_entities {
                                        if key == &"extension" {
                                            continue;
                                        }
                                        if let Some(foo) = path_entities.get(key) {
                                            if foo != val {
                                                return false;
                                            }
                                        } else {
                                            return false;
                                        }
                                    }
                                    true
                                })
                                .collect::<HashSet<_>>();
                            for (key, val) in md.read_as_metadata()? {
                                md_builder.add_entry(&key, &val, &ixs);
                            }
                        }

                        Ok(())
                    }();
                }
            }
            // let len = sub.len();
        }
        md_builder
    }
}
