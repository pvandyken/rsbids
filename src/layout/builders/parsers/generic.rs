use crate::{
    layout::{
        bidspath::{BidsPath, UnknownDatatype, UnknownDatatypeTypes},
        builders::{
            bidspath_builder::{BidsPathBuilder, BidsPathPart, Name},
            primitives::ComponentType,
            LayoutBuilder,
        },
        check_datatype,
        entity_table::EntityTable,
    },
    standards::BIDS_ENTITIES,
};

impl BidsPathBuilder {
    pub fn generic_build_parse(
        self,
        ds_builder: &mut LayoutBuilder,
    ) -> BidsPath {
        let is_twotype: Vec<bool> = self
            .components
            .iter()
            .map(|comp| matches!(comp, ComponentType::TwoType(..)))
            .collect();
        let mut labelled = Vec::new();

        let len = self.components.len();
        for (i, comp) in self.components.into_iter().enumerate() {
            if i + 1 == len {
                labelled.push(BidsPathPart::Name(match comp {
                    ComponentType::OneType(keyval) => Name::from_onetype(keyval),
                    ComponentType::TwoType(elems) => Name::from_twotype(elems),
                    ComponentType::ZeroType(comp) => Name::from_zerotype(comp),
                }));
                break;
            }
            let next_is_twotype = is_twotype[i + 1];
            labelled.push(Self::label_component_type(
                labelled.last().unwrap_or(&BidsPathPart::Head(0)),
                comp,
                &self.path.as_str(),
                next_is_twotype,
                &ds_builder.entities,
            ));
        }
        Self::collect_elements(
            ds_builder,
            BidsPath::new(self.path, self.root, self.depth),
            labelled,
        )

        // (BidsPath::new(self.path, self.root), labelled)
    }

    fn check_entity(entity: &str, known_entities: &EntityTable<String>) -> bool {
        known_entities.contains_key(entity) || BIDS_ENTITIES.contains_left(entity)
    }
    fn label_component_type<'b>(
        previous: &BidsPathPart,
        comp: ComponentType,
        template: &str,
        next_is_twotype: bool,
        known_entities: &EntityTable<String>,
    ) -> BidsPathPart {
        match comp {
            ComponentType::TwoType(elems) => BidsPathPart::Name(Name::from_twotype(elems)),
            ComponentType::OneType(keyval) => match previous {
                BidsPathPart::Head(..) => {
                    if Self::check_entity(keyval.get_key(template), known_entities) {
                        BidsPathPart::Parent(keyval)
                    } else {
                        BidsPathPart::UncertainParent(keyval)
                    }
                }
                BidsPathPart::Datatype(..) | BidsPathPart::Name(..) => {
                    BidsPathPart::Name(Name::from_onetype(keyval))
                }
                BidsPathPart::Parent(..) => BidsPathPart::Parent(keyval),
                BidsPathPart::UncertainParent(..) | BidsPathPart::UncertainDatatype(..) => {
                    BidsPathPart::UncertainParent(keyval)
                }
            },
            ComponentType::ZeroType(comp) => match previous {
                BidsPathPart::Head(..) => {
                    if next_is_twotype || check_datatype(&template[comp.clone()]) {
                        BidsPathPart::Datatype(comp)
                    } else {
                        BidsPathPart::Head(comp.end)
                    }
                }
                BidsPathPart::Datatype(..) | BidsPathPart::Name(..) => {
                    BidsPathPart::Name(Name::from_zerotype(comp))
                }
                BidsPathPart::Parent(..) => BidsPathPart::Datatype(comp),
                BidsPathPart::UncertainDatatype(..) => {
                    let is_valid = next_is_twotype || check_datatype(&template[comp.clone()]);
                    BidsPathPart::UncertainDatatype(UnknownDatatypeTypes::Unlinked(
                        UnknownDatatype::new(comp, is_valid),
                    ))
                }
                BidsPathPart::UncertainParent(keyval) => {
                    let is_valid = next_is_twotype || check_datatype(&template[comp.clone()]);
                    BidsPathPart::UncertainDatatype(UnknownDatatypeTypes::Linked(
                        keyval.get_key(template).to_string(),
                        UnknownDatatype::new(comp, is_valid),
                    ))
                }
            },
        }
    }

    fn collect_elements(
        ds_builder: &mut LayoutBuilder,
        mut path: BidsPath,
        parts: Vec<BidsPathPart>,
    ) -> BidsPath {
        for (i, part) in parts.into_iter().rev().enumerate() {
            match part {
                BidsPathPart::Head(i) => {
                    if path.head == 0 {
                        path.head = i;
                        if path.root > i {
                            path.root = i
                        }
                    }
                }
                BidsPathPart::Parent(keyval) => {
                    let (key, value) = keyval.get(&path.as_str());
                    ds_builder.add_and_confirm_entity(key, value);
                    path.parents.push(keyval)
                }
                BidsPathPart::UncertainParent(keyval) => {
                    let (key, value) = keyval.get(&path.as_str());
                    ds_builder.add_entity(key, value);
                    path.add_uncertain_parent(keyval)
                }
                BidsPathPart::Datatype(comp) => {
                    ds_builder.add_entity("datatype", &path[&comp]);
                    path.datatype = Some(comp)
                }
                BidsPathPart::Name(mut name) => {
                    if let Some(parts) = name.parts {
                        path.extend_parts(parts)
                    }
                    if i == 0 {
                        if let Some(mut suffix) = name.suffix {
                            if let Some(extension) = path.extract_extension(&mut suffix) {
                                ds_builder.add_entity("extension", &path[&extension]);
                                path.extension = Some(extension);
                            }
                            ds_builder.add_entity("suffix", &path[&suffix]);
                            path.suffix = Some(suffix)
                        } else if let Some(keyval) = name.entities.as_mut().and_then(|kv| kv.pop())
                        {
                            if let Some(extension) = path.extract_extension(&mut keyval.val_range())
                            {
                                ds_builder.add_entity("extension", &path[&extension]);
                                path.extension = Some(extension);
                            }
                            let (key, value) = keyval.get(&path.as_str());
                            ds_builder.add_and_confirm_entity(key, value);
                            path.entities.push(keyval);
                        }
                    } else if let Some(suffix) = name.suffix {
                        path.push_part(suffix)
                    }
                    if let Some(entities) = name.entities {
                        for entity in entities {
                            let (key, value) = entity.get(&path.as_str());
                            ds_builder.add_and_confirm_entity(key, value);
                            path.entities.push(entity);
                        }
                    }
                }
                BidsPathPart::UncertainDatatype(datatype) => {
                    ds_builder.add_uncertain_datatype();
                    path.push_uncertain_datatype(datatype)
                }
            }
        }
        path
    }
}
