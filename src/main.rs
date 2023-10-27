use std::{
    env::{self},
    ffi::OsString,
    path::PathBuf,
    process::exit,
};

use async_walkdir::Filtering;
use futures_lite::{future::block_on, StreamExt};
use itertools::Itertools;
use rsbids::dataset::Dataset;
use walkdir::WalkDir;

fn main() {
    let args: Vec<String> = env::args().dropping(1).collect();
    if args.len() == 0 {
        eprintln!("No arguments given!");
        exit(1)
    }
    let mut dataset = Dataset::default();
    for elem in args {
        match PathBuf::from(elem).canonicalize() {
            Ok(elem) => match iterdir(elem, |path| dataset.add_path(path)) {
                Ok(..) => (),
                Err(e) => eprintln!("error parsing path: {}", e.to_string_lossy()),
            },
            Err(e) => eprintln!("error: {}", e),
        }
    }
    dataset.cleanup();
    dbg!(dataset);
}
