use std::ops::Range;

use crate::{
    errors::BidsPathErr,
    layout::{
        bidspath::BidsPath,
        builders::{
            bidspath_builder::BidsPathBuilder,
            primitives::{ComponentType, Elements, KeyVal},
        },
        check_datatype,
    },
    standards::check_entity as spec_check_entity,
};

struct TemplateParser<I: Fn(&str) -> bool> {
    bidspath: BidsPath,
    check_entity: I,
}

impl<I: Fn(&str) -> bool> TemplateParser<I> {
    #[inline]
    fn finalize(&mut self) {
        if self.bidspath.root > self.bidspath.head {
            self.bidspath.root = self.bidspath.head
        }
    }

    fn handle_twotype(&mut self, elems: Vec<Elements>, last_component: bool) -> Result<(), ()> {
        for (j, elem) in elems.into_iter().rev().enumerate() {
            if j == 0 && last_component {
                match elem {
                    Elements::Suffix(mut range) => {
                        if let Some(extension) = self.bidspath.extract_extension(&mut range) {
                            self.bidspath.extension = Some(extension);
                        }
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
                        if (self.check_entity)(keyval.get_key(&self.bidspath.as_str())) {
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
        if check_datatype(&self.bidspath.as_str()[range.clone()]) {
            self.bidspath.datatype = Some(range.clone());
            Some(LastMatch::Datatype)
        } else {
            None
        }
    }

    fn handle_keyval(&mut self, keyval: KeyVal) -> Option<LastMatch> {
        if (self.check_entity)(keyval.get_key(&self.bidspath.as_str())) {
            self.bidspath.parents.push(keyval.clone());
            Some(LastMatch::Parent)
        } else {
            None
        }
    }

    fn handle_head(&mut self, range: Range<usize>) -> LastMatch {
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

impl BidsPathBuilder {
    #[inline]
    pub fn spec_parse(self) -> Result<BidsPath, BidsPathErr> {
        self.template_parse(spec_check_entity)
    }

    pub fn template_parse<I: Fn(&str) -> bool>(
        self,
        check_entity: I,
    ) -> Result<BidsPath, BidsPathErr> {
        let bidspath = BidsPath::new(self.path, self.root, self.depth);
        let mut lastmatch = LastMatch::Head;
        let len = self.components.len();
        let mut parser = TemplateParser {
            bidspath,
            check_entity,
        };
        for (i, comp) in self.components.into_iter().enumerate() {
            // Last component
            if i + 1 == len {
                match comp {
                    ComponentType::OneType(..) | ComponentType::ZeroType(..) => {
                        return Err(BidsPathErr::Validation(parser.bidspath.clear()));
                    }
                    ComponentType::TwoType(elems) => {
                        if let Err(_) = parser.handle_twotype(elems, true) {
                            return Err(BidsPathErr::Validation(parser.bidspath.clear()));
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
                            return Err(BidsPathErr::Validation(parser.bidspath.clear()));
                        }
                    }
                }
            }
        }
        parser.finalize();
        Ok(parser.bidspath)
    }
}
