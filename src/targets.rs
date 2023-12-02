use anyhow::anyhow;

use crate::{
    controller::Controller,
    protogen,
    types::{Subtype, Type},
};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SpellTarget {
    pub controller: Controller,
    pub types: Vec<Type>,
    pub subtypes: Vec<Subtype>,
}

impl TryFrom<&protogen::targets::SpellTarget> for SpellTarget {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::targets::SpellTarget) -> Result<Self, Self::Error> {
        Ok(Self {
            controller: value
                .controller
                .controller
                .as_ref()
                .map(Controller::from)
                .unwrap_or_default(),
            types: value
                .types
                .iter()
                .map(|ty| {
                    ty.ty
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected type to have a type set"))
                        .map(Type::from)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
            subtypes: value
                .subtypes
                .iter()
                .map(|ty| {
                    ty.subtype
                        .as_ref()
                        .ok_or_else(|| anyhow!("Expected type to have a type set"))
                        .map(Subtype::from)
                })
                .collect::<anyhow::Result<Vec<_>>>()?,
        })
    }
}
