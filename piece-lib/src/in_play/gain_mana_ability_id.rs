use uuid::Uuid;

use crate::{
    in_play::Database,
    protogen::{
        effects::GainManaAbility,
        ids::{CardId, GainManaAbilityId},
    },
};

#[derive(Debug)]
pub struct GainManaAbilityInPlay {
    pub(crate) source: CardId,
    pub(crate) ability: GainManaAbility,
}

impl GainManaAbilityId {
    pub(crate) fn generate() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            ..Default::default()
        }
    }

    pub(crate) fn upload(db: &mut Database, source: CardId, ability: GainManaAbility) -> Self {
        let id = Self::generate();

        db.mana_abilities
            .insert(id.clone(), GainManaAbilityInPlay { source, ability });

        id
    }
}
