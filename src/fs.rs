use std::{ffi::OsString, io, path::PathBuf};

use itertools::Itertools;
// use async_walkdir::Filtering;
// use futures_lite::{future::block_on, StreamExt};
use walkdir::WalkDir;

fn unicode_decode_err(os_string: OsString) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Unable to convert path to unicode: {}",
            os_string.to_string_lossy()
        ),
    )
}

pub fn iterdir<F: FnMut(String)>(path: PathBuf, mut callback: F) -> Result<(), io::Error> {
    if path.is_file() {
        return Ok(callback(
            path.into_os_string()
                .into_string()
                .map_err(|e| unicode_decode_err(e))?,
        ));
    } else {
        WalkDir::new(path)
            .into_iter()
            .filter_entry(|entry| {
                // dbg!(&entry);
                if let Some(true) = entry
                    .path()
                    .file_name()
                    .map(|f| f.to_string_lossy() == "dataset_description.json")
                {
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
                        if entry.path().is_file() {
                            match entry.path().to_str() {
                                Some(path) => callback(path.to_string()),
                                None => {
                                    return Err(io::Error::new(io::ErrorKind::InvalidData, "Error"))
                                }
                            };
                        }
                    }
                    Err(e) => {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
                    }
                };
                Ok(())
            })
            .collect_vec();
        Ok(())
    }
    // block_on(async {
    //     let mut entries = async_walkdir::WalkDir::new(path).filter(|entry| async move {
    //         if !entry.path().is_file() {
    //             Filtering::Ignore
    //         } else if let Some(true) = entry
    //             .path()
    //             .file_name()
    //             .map(|f| f.to_string_lossy() == "dataset_description.json")
    //         {
    //             Filtering::Ignore
    //         } else if let Some(true) = entry
    //             .path()
    //             .file_name()
    //             .map(|f| f.to_string_lossy().starts_with('.'))
    //         {
    //             return Filtering::IgnoreDir;
    //         } else {
    //             Filtering::Continue
    //         }
    //     });
    //     loop {
    //         match entries.next().await {
    //             Some(Ok(entry)) => match entry.path().into_os_string().into_string() {
    //                 Ok(path) => callback(path),
    //                 Err(e) => return Err(unicode_decode_err(e)),
    //             },
    //             Some(Err(e)) => return Err(e),
    //             None => break,
    //         };
    //     }
    // })
}
// fn main() {
//     let args: Vec<String> = env::args().dropping(1).collect();
//     if args.len() == 0 {
//         eprintln!("No arguments given!");
//         exit(1)
//     }
//     let mut dataset = Dataset::default();
//     for elem in args {
//         match PathBuf::from(elem).canonicalize() {
//             Ok(elem) => match iterdir(elem, |path| dataset.add_path(path)) {
//                 Ok(..) => (),
//                 Err(e) => eprintln!("error parsing path: {}", e),
//             },
//             Err(e) => eprintln!("error: {}", e),
//         }
//     }
//     dataset.cleanup();
//     dbg!(dataset);
// }
