use std::{
    env::{self},
    process::exit, path::PathBuf,
};

use itertools::Itertools;
use rsbids::layout::Layout;

fn main() {
    let args: Vec<_> = env::args().dropping(1).map(PathBuf::from).collect();
    if args.len() == 0 {
        eprintln!("No arguments given!");
        exit(1)
    }
    let _ = Layout::create(args, None, false);
}
