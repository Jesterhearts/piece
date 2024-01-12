use std::sync::atomic::Ordering;

use derive_more::{From, Into};

use crate::{
    in_play::{CardId, Database, NEXT_ABILITY_ID},
    protogen::effects::ActivatedAbility,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub struct ActivatedAbilityId(usize);

#[derive(Debug)]
pub struct ActivatedAbilityInPlay {
    pub(crate) source: CardId,
    pub(crate) ability: ActivatedAbility,
}

impl ActivatedAbilityId {
    pub(crate) fn new() -> Self {
        Self(NEXT_ABILITY_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub(crate) fn upload(db: &mut Database, source: CardId, ability: ActivatedAbility) -> Self {
        let id = Self::new();

        db.activated_abilities
            .insert(id, ActivatedAbilityInPlay { source, ability });

        id
    }
}
