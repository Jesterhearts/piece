use crate::protogen;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Type {
    BasicLand,
    Land,
    Instant,
    Sorcery,
    Creature,
    Artifact,
    Enchantment,
    Battle,
}

impl From<&protogen::types::type_::Ty> for Type {
    fn from(value: &protogen::types::type_::Ty) -> Self {
        match value {
            protogen::types::type_::Ty::BasicLand(_) => Self::BasicLand,
            protogen::types::type_::Ty::Land(_) => Self::Land,
            protogen::types::type_::Ty::Instant(_) => Self::Instant,
            protogen::types::type_::Ty::Sorcery(_) => Self::Sorcery,
            protogen::types::type_::Ty::Creature(_) => Self::Creature,
            protogen::types::type_::Ty::Artifact(_) => Self::Artifact,
            protogen::types::type_::Ty::Enchantment(_) => Self::Enchantment,
            protogen::types::type_::Ty::Battle(_) => Self::Battle,
        }
    }
}

impl Type {
    pub fn is_permanent(&self) -> bool {
        !matches!(self, Type::Instant | Type::Sorcery)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Subtype {
    Bear,
    Elf,
    Shaman,
    Plains,
    Island,
    Swamp,
    Mountain,
    Forest,
}

impl From<&protogen::types::subtype::Subtype> for Subtype {
    fn from(value: &protogen::types::subtype::Subtype) -> Self {
        match value {
            protogen::types::subtype::Subtype::Bear(_) => Self::Bear,
            protogen::types::subtype::Subtype::Elf(_) => Self::Elf,
            protogen::types::subtype::Subtype::Shaman(_) => Self::Shaman,
            protogen::types::subtype::Subtype::Plains(_) => Self::Plains,
            protogen::types::subtype::Subtype::Island(_) => Self::Island,
            protogen::types::subtype::Subtype::Swamp(_) => Self::Swamp,
            protogen::types::subtype::Subtype::Mountain(_) => Self::Mountain,
            protogen::types::subtype::Subtype::Forest(_) => Self::Forest,
        }
    }
}
