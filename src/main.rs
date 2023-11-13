use std::{
    env::{self},
    process::exit,
};

use itertools::Itertools;
use rsbids::dataset::Dataset;

fn main() {
    let args: Vec<String> = env::args().dropping(1).collect();
    if args.len() == 0 {
        eprintln!("No arguments given!");
        exit(1)
    }
    let _ = Dataset::create(args, None, false);
}
