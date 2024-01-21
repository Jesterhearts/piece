use uuid::Uuid;

use crate::{
    in_play::Database,
    protogen::{
        effects::ActivatedAbility,
        ids::{ActivatedAbilityId, CardId},
    },
};

#[derive(Debug)]
pub struct ActivatedAbilityInPlay {
    pub(crate) source: CardId,
    pub(crate) ability: ActivatedAbility,
}

impl ActivatedAbilityId {
    pub(crate) fn generate() -> Self {
        let (hi, lo) = Uuid::new_v4().as_u64_pair();
        Self {
            hi,
            lo,
            ..Default::default()
        }
    }

    pub(crate) fn upload(db: &mut Database, source: CardId, ability: ActivatedAbility) -> Self {
        let id = Self::generate();

        db.activated_abilities
            .insert(id.clone(), ActivatedAbilityInPlay { source, ability });

        id
    }
}
