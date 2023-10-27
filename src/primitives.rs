use std::ops::Range;

#[derive(Debug)]
pub struct Slice(pub usize, pub usize);

impl Slice {
    pub fn get<'a>(&self, template: &'a str) -> &'a str {
        &template[self.0..self.1]
    }
}

#[derive(Debug)]
pub struct KeyVal {
    pub slice: Range<usize>,
    pub delimiter: usize,
}

impl KeyVal {
    pub fn new(slice: Range<usize>, delim: usize) -> KeyVal {
        KeyVal {
            slice,
            delimiter: delim,
        }
    }

    pub fn key_range(&self) -> Range<usize> {
        self.slice.start..self.delimiter
    }

    pub fn val_range(&self) -> Range<usize> {
        self.delimiter + 1..self.slice.end
    }

    pub fn get<'a>(&self, template: &'a str) -> (&'a str, &'a str) {
        (self.get_key(template), self.value(template))
    }

    pub fn get_key<'a>(&self, template: &'a str) -> &'a str {
        &template[self.key_range()]
    }

    pub fn value<'a>(&self, template: &'a str) -> &'a str {
        &template[self.val_range()]
    }
}

#[derive(Debug)]
pub enum ComponentType {
    ZeroType(Range<usize>),
    OneType(KeyVal),
    TwoType(Vec<Elements>),
}

#[derive(Debug)]
pub enum Elements {
    KeyVal(KeyVal),
    Suffix(Range<usize>),
    Part(Range<usize>),
}
#[derive(Debug)]
pub enum Primitive {
    Value(usize, usize),
    Key(usize, usize),
    Suffix(usize, usize),
    Part(usize, usize),
}


#[derive(Debug)]
pub enum PrePrimitive {
    Prefix,
    KeyLike(usize),
    ValueLike(usize),
}
