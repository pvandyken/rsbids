use once_cell::sync::Lazy;
use std::collections::HashSet;

pub static BIDS_ENTITIES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    {
        [
            "sub",
            "ses",
            "datatype",
            "extension",
            "suffix",
            "sample",
            "task",
            "acq",
            "ce",
            "stain",
            "trc",
            "rec",
            "dir",
            "run",
            "proc",
            "mod",
            "echo",
            "flip",
            "inv",
            "mt",
            "part",
            "recording",
            "space",
            "chunk",
            "null",
            "null",
            "null",
            "split",
            "atlas",
            "roi",
            "label",
            "from",
            "to",
            "mode",
            "hemi",
            "res",
            "den",
            "model",
            "subset",
            "desc",
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
