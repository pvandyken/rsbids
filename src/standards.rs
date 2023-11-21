use bimap::BiMap;
use once_cell::sync::Lazy;
use std::collections::HashSet;

pub fn get_key_alias(key: &str) -> &str {
    match BIDS_ENTITIES.get_by_left(key) {
        Some(key) => key,
        None => key,
    }
}

pub fn deref_key_alias(key: &str) -> Option<&str> {
     BIDS_ENTITIES.get_by_right(key).copied()
}

pub fn check_entity(entity: &str) -> bool {
    BIDS_ENTITIES.contains_left(entity)
}

pub static BIDS_ENTITIES: Lazy<BiMap<&'static str, &'static str>> = Lazy::new(|| {
    {
        [
            ("sub", "subject"),
            ("ses", "session"),
            ("datatype", "datatype"),
            ("extension", "extension"),
            ("suffix", "suffix"),
            ("sample", "sample"),
            ("task", "task"),
            ("tracksys", "tracksys"),
            ("acq", "acquisition"),
            ("ce", "ceagent"),
            ("stain", "staining"),
            ("trc", "tracer"),
            ("rec", "reconstruction"),
            ("dir", "direction"),
            ("run", "run"),
            ("proc", "proc"),
            ("mod", "modality"),
            ("echo", "echo"),
            ("flip", "flip"),
            ("inv", "inv"),
            ("mt", "mt"),
            ("part", "part"),
            ("recording", "recording"),
            ("space", "space"),
            ("chunk", "chunk"),
            ("split", "split"),
            ("atlas", "atlas"),
            ("roi", "roi"),
            ("label", "label"),
            ("from", "from"),
            ("to", "to"),
            ("mode", "mode"),
            ("hemi", "hemisphere"),
            ("res", "res"),
            ("den", "density"),
            ("model", "model"),
            ("subset", "subset"),
            ("desc", "description"),
        ]
    }
    .iter()
    .cloned()
    .collect()
});

pub static BIDS_DATATYPES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "anat", "beh", "dwi", "eeg", "fmap", "func", "ieeg", "meg", "motion", "micr", "nirs",
        "perf", "pet",
    ]
    .iter()
    .cloned()
    .collect()
});
