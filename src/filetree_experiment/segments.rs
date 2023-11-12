use crate::primitives::{ComponentType, Primitive, PrePrimitive, Elements, KeyVal};

fn get_next_elem(path: &str) -> PrePrimitive {
    match path.rfind(['_', '-']) {
        Some(i) => match path.as_bytes()[i] {
            b'_' => PrePrimitive::KeyLike(i),
            b'-' => PrePrimitive::ValueLike(i),
            _ => panic!("Unexpected match found!"),
        },
        None => PrePrimitive::Prefix,
    }
}

fn process_keylike(start: usize, end: usize, last_elem: Option<&Primitive>) -> Primitive {
    match last_elem {
        Some(last) => match last {
            Primitive::Suffix(..) => Primitive::Suffix(start, end),
            Primitive::Value(..) => Primitive::Key(start, end),
            Primitive::Key(..) | Primitive::Part(..) => Primitive::Part(start, end),
        },
        None => Primitive::Suffix(start, end),
    }
}

pub fn parse_path_segment<'a>(segment: &str) -> Vec<Primitive> {
    fn recurse(path: &str, start_i: usize, offset: usize, elems: &mut Vec<Primitive>) {
        let mut next_i = 0;
        let mut finish = false;
        let next_elem = match get_next_elem(&path[..start_i]) {
            PrePrimitive::KeyLike(i) => {
                next_i = i;
                process_keylike(i + offset + 1, start_i + offset, elems.last())
            }
            PrePrimitive::ValueLike(i) => {
                next_i = i;
                Primitive::Value(i + offset + 1, start_i + offset)
            }
            PrePrimitive::Prefix => {
                finish = true;
                process_keylike(offset, start_i + offset, elems.last())
            }
        };
        elems.push(next_elem);
        if finish {
            return;
        } else {
            recurse(path, next_i, offset, elems)
        }
    }
    let mut primitives = Vec::new();
    let start = 0;
    let len = segment.len();
    recurse(&segment, len, start, &mut primitives);
    primitives
}

pub fn classify_component<'a>(mut elements: Vec<Elements>) -> ComponentType {
    if elements.len() > 1 {
        return ComponentType::TwoType(elements);
    }
    match elements.pop().expect("Should have at least one element") {
        Elements::Suffix(suffix) => ComponentType::ZeroType(suffix),
        Elements::KeyVal(keyval) => ComponentType::OneType(keyval),
        Elements::Part(..) => panic!("Should not have a part as the only element in a component"),
    }
}


fn consume_values<'a>(data: &mut Vec<Primitive>, keystart: usize, keyend: usize) -> Elements {
    let mut end = keyend + 1;
    while let Some(last) = data.pop() {
        match last {
            Primitive::Value(_s, e) => end = e,
            _ => {
                panic!("Should not have encountered a non-value within the consume_values loop")
            }
        }
        if data
            .last()
            .is_some_and(|next| -> bool { !matches!(next, Primitive::Value(..)) })
        {
            break;
        }
    }
    Elements::KeyVal(KeyVal::new(keystart..end, keyend))
}

pub fn group_primitives(mut data: Vec<Primitive>) -> Vec<Elements> {
    let mut elems = Vec::new();
    while let Some(last) = data.pop() {
        let mut finish = false;
        let grouped = match last {
            Primitive::Key(start, end) => consume_values(&mut data, start, end),
            Primitive::Suffix(start, _end) => {
                finish = true;
                Elements::Suffix(start.._end)
            }
            Primitive::Part(start, end) => Elements::Part(start..end),
            Primitive::Value(..) => {
                panic!("Values should have been consumed within the key block")
            }
        };
        elems.push(grouped);
        if finish {
            break;
        }
    }
    elems
}

pub struct BidsPathComponents {
    pub components: Vec<ComponentType>,
}
