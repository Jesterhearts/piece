use uuid::Uuid;

use crate::{
    in_play::{CardId, Database},
    protogen::effects::GainManaAbility,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct GainManaAbilityId(Uuid);

#[derive(Debug)]
pub struct GainManaAbilityInPlay {
    pub(crate) source: CardId,
    pub(crate) ability: GainManaAbility,
}

impl GainManaAbilityId {
    pub(crate) fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub(crate) fn upload(db: &mut Database, source: CardId, ability: GainManaAbility) -> Self {
        let id = Self::new();

        db.mana_abilities
            .insert(id, GainManaAbilityInPlay { source, ability });

        id
    }
}
