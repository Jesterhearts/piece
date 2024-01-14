use derive_more::{From, Into};
use uuid::Uuid;

use crate::{
    in_play::{CardId, Database},
    protogen::effects::ActivatedAbility,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub struct ActivatedAbilityId(Uuid);

#[derive(Debug)]
pub struct ActivatedAbilityInPlay {
    pub(crate) source: CardId,
    pub(crate) ability: ActivatedAbility,
}

impl ActivatedAbilityId {
    pub(crate) fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub(crate) fn upload(db: &mut Database, source: CardId, ability: ActivatedAbility) -> Self {
        let id = Self::new();

        db.activated_abilities
            .insert(id, ActivatedAbilityInPlay { source, ability });

        id
    }
}
