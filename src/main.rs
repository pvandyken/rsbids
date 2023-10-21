use std::env;

// fn get_key_val(elem: &str) -> Option<(usize, usize)> {
//     let key_ix = elem.find("-")?;
//     let val_ix = elem[key_ix..].find("_").unwrap_or(elem[key_ix..].len());
//     Some((key_ix, val_ix))
// }

// fn add_key_if<'a>(entities: &mut HashMap<String, &'a str>, key: String, value: &'a str) {
//     if value.len() > 0 {
//         entities.insert(key, value);
//     }
// }

// fn add_suffix_if<'a>(entities: &mut HashMap<String, &'a str>, suffix: &'a str) {
//     add_key_if(entities, String::from("suffix"), suffix);
// }

// fn check_entity_dir(entities: &HashMap<String, &str>, path: &Path) -> bool {
//     let parent = path
//         .file_name()
//         .expect("file_name must not end in ..")
//         .to_str()
//         .expect("path must be valid unicode");
//     if let Some((key, val)) = parent.split_once("-") {
//         if entities.get(key).is_some_and(|v| v == &val) {
//             true
//         } else {
//             false
//         }
//     } else {
//         false
//     }
// }

// struct TwoTypeParts<'a> {
//     map: HashMap<&'a str, &'a str>,
//     prefix: Option<&'a str>,
//     suffix: Option<&'a str>,
// }

enum BidsTypes<'a> {
    KeyVal(&'a str, &'a str),
    Suffix(&'a str),
    Part(&'a str),
    Prefix(&'a str),
}
enum PathCompTypes {
    Value(usize, usize),
    Key(usize, usize),
    Suffix(usize, usize),
    Prefix(usize, usize),
    Part(usize, usize),
}

#[derive(Debug)]
enum ElemTypes {
    Suffix,
    KeyLike(usize),
    ValueLike(usize),
}

fn get_next_elem(path: &str) -> ElemTypes {
    match path.find(['_', '-']) {
        Some(i) => match path.as_bytes()[i] {
            b'-' => ElemTypes::KeyLike(i),
            b'_' => ElemTypes::ValueLike(i),
            _ => panic!("Unexpected match found!"),
        },
        None => ElemTypes::Suffix,
    }
}

fn parse_path_segment(path: &str, start_i: usize) -> Vec<PathCompTypes> {
    let mut elems: Vec<PathCompTypes> = Vec::new();
    let mut next_i = 0;
    let mut finish = false;
    let next_elem = match get_next_elem(path) {
        ElemTypes::Suffix => {
            finish = true;
            match elems.last() {
                Some(last) => match last {
                    PathCompTypes::Key(..) => PathCompTypes::Value(start_i, path.len()),
                    PathCompTypes::Value(..)
                    | PathCompTypes::Prefix(..)
                    | PathCompTypes::Part(..) => PathCompTypes::Suffix(start_i, path.len()),
                    _ => panic!("Suffix should not be following suffix!"),
                },
                None => PathCompTypes::Part(start_i, path.len()),
            }
        }
        ElemTypes::KeyLike(i) => {
            next_i = i;
            if elems
                .last()
                .is_some_and(|val| -> bool { matches!(val, PathCompTypes::Key(..)) })
            {
                PathCompTypes::Value(start_i, i)
            } else {
                PathCompTypes::Key(start_i, i)
            }
        }
        ElemTypes::ValueLike(i) => {
            next_i = i;
            match elems.last() {
                Some(last) => match last {
                    PathCompTypes::Value(..)
                    | PathCompTypes::Prefix(..)
                    | PathCompTypes::Part(..) => PathCompTypes::Part(start_i, i),
                    PathCompTypes::Key(..) => PathCompTypes::Key(start_i, i),
                    _ => panic!("ValueLike should not be following suffix!"),
                },
                None => PathCompTypes::Prefix(start_i, i),
            }
        }
    };
    elems.push(next_elem);
    if finish {
        elems
    } else {
        parse_path_segment(path, next_i)
    }
}

fn parse(path: &str) {
    let parsed = parse_path_segment(path, 0);


}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1) {
        Some(elem) => {
            let entities = get_next_elem(elem.as_str());
            dbg!(entities);
        }
        None => println!("No argument given"),
    }
}
