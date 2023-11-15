use std::ops::Range;

use crate::{
    layout::{
        bidspath::BidsPath,
        builders::{
            bidspath_builder::BidsPathBuilder,
            primitives::{ComponentType, Elements, KeyVal},
            LayoutBuilder,
        },
        check_datatype,
    },
    standards::BIDS_ENTITIES,
};

struct SpecParser<'a> {
    bidspath: BidsPath,
    ds_builder: Option<&'a mut LayoutBuilder>,
}

impl SpecParser<'_> {
    #[inline]
    fn add_entity(&mut self, entity: &str, range: &Range<usize>) {
        if let Some(builder) = &mut self.ds_builder.as_mut() {
            builder.add_entity(entity, &self.bidspath.path[range.clone()])
        }
    }

    #[inline]
    fn add_keyval(&mut self, keyval: &KeyVal) {
        let entity = keyval.get_key(&self.bidspath.path);
        let value = keyval.value(&self.bidspath.path);
        if let Some(builder) = &mut self.ds_builder.as_mut() {
            builder.add_entity(entity, value)
        }
    }

    #[inline]
    fn add_head(&mut self, head: &Range<usize>) {
        if let Some(builder) = &mut self.ds_builder.as_mut() {
            builder.add_head(&self.bidspath.path[head.clone()])
        }
    }
    fn handle_twotype(&mut self, elems: Vec<Elements>, last_component: bool) -> Result<(), ()> {
        for (j, elem) in elems.into_iter().rev().enumerate() {
            if j == 0 && last_component {
                match elem {
                    Elements::Suffix(mut range) => {
                        if let Some(extension) = self.bidspath.extract_extension(&mut range) {
                            self.add_entity("extension", &extension);
                            self.bidspath.extension = Some(extension);
                        }
                        self.add_entity("suffix", &range);
                        self.bidspath.suffix = Some(range);
                    }
                    _ => {
                        // Very last element must be suffix
                        return Err(());
                    }
                }
            } else {
                match elem {
                    Elements::KeyVal(keyval) => {
                        if check_entity(keyval.get_key(&self.bidspath.path)) {
                            self.add_keyval(&keyval);
                            self.bidspath.parents.push(keyval.clone());
                        } else {
                            self.bidspath.push_part(keyval.slice.clone());
                        }
                    }
                    Elements::Part(range) | Elements::Suffix(range) => {
                        self.bidspath.push_part(range)
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_datatype(&mut self, range: Range<usize>) -> Option<LastMatch> {
        if check_datatype(&self.bidspath.path[range.clone()]) {
            self.add_entity("datatype", &range);
            self.bidspath.datatype = Some(range.clone());
            Some(LastMatch::Datatype)
        } else {
            None
        }
    }

    fn handle_keyval(&mut self, keyval: KeyVal) -> Option<LastMatch> {
        if check_entity(keyval.get_key(&self.bidspath.path)) {
            self.add_keyval(&keyval);
            self.bidspath.parents.push(keyval.clone());
            Some(LastMatch::Parent)
        } else {
            None
        }
    }

    fn handle_head(&mut self, range: Range<usize>) -> LastMatch {
        self.add_head(&range);
        self.bidspath.head = range.end;
        LastMatch::Head
    }

    fn handle_part(&mut self, range: Range<usize>) -> LastMatch {
        self.bidspath.push_part(range);
        LastMatch::Name
    }
}

enum LastMatch {
    Head,
    Parent,
    Datatype,
    Name,
}

fn check_entity(entity: &str) -> bool {
    BIDS_ENTITIES.contains_left(entity)
}
impl BidsPathBuilder {
    pub fn spec_parse(self, ds_builder: Option<&mut LayoutBuilder>) -> Result<BidsPath, Self> {
        let bidspath = BidsPath::new(self.path, self.root);
        let mut lastmatch = LastMatch::Head;
        let len = self.components.len();
        let mut parser = SpecParser {
            bidspath,
            ds_builder,
        };
        for (i, comp) in self.components.into_iter().enumerate() {
            // Last component
            if i + 1 == len {
                match comp {
                    ComponentType::OneType(..) | ComponentType::ZeroType(..) => {
                        let bidspath = parser.bidspath;
                        return Err(Self::new(bidspath.path, bidspath.root));
                    }
                    ComponentType::TwoType(elems) => {
                        if let Err(_) = parser.handle_twotype(elems, true) {
                            return Err(Self::new(parser.bidspath.path, parser.bidspath.root));
                        }
                    }
                }
            } else {
                match comp {
                    ComponentType::ZeroType(range) => match lastmatch {
                        LastMatch::Head => {
                            lastmatch = parser
                                .handle_datatype(range.clone())
                                .unwrap_or_else(|| parser.handle_head(range));
                        }
                        LastMatch::Parent => {
                            lastmatch = parser
                                .handle_datatype(range.clone())
                                .unwrap_or_else(|| parser.handle_part(range));
                        }
                        LastMatch::Datatype | LastMatch::Name => {
                            lastmatch = parser.handle_part(range);
                        }
                    },
                    ComponentType::OneType(keyval) => match lastmatch {
                        LastMatch::Head => {
                            lastmatch = parser
                                .handle_keyval(keyval.clone())
                                .unwrap_or_else(|| parser.handle_head(keyval.slice))
                        }
                        LastMatch::Parent => {
                            lastmatch = parser
                                .handle_keyval(keyval.clone())
                                .unwrap_or_else(|| parser.handle_part(keyval.slice))
                        }
                        LastMatch::Datatype | LastMatch::Name => {
                            lastmatch = parser
                                .handle_keyval(keyval.clone())
                                .unwrap_or_else(|| parser.handle_part(keyval.slice))
                        }
                    },
                    ComponentType::TwoType(elems) => {
                        if let Err(_) = parser.handle_twotype(elems, false) {
                            return Err(Self::new(parser.bidspath.path, parser.bidspath.root));
                        }
                    }
                }
            }
        }
        Ok(parser.bidspath)
    }
}
