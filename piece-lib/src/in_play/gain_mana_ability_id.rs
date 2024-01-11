use std::sync::atomic::Ordering;

use derive_more::{From, Into};

use crate::{
    abilities::GainManaAbility,
    in_play::{CardId, Database, NEXT_ABILITY_ID},
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub struct GainManaAbilityId(usize);

#[derive(Debug)]
pub struct GainManaAbilityInPlay {
    pub(crate) source: CardId,
    pub(crate) ability: GainManaAbility,
    pub(crate) temporary: bool,
}

impl GainManaAbilityId {
    pub(crate) fn new() -> Self {
        Self(NEXT_ABILITY_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub(crate) fn upload(db: &mut Database, source: CardId, ability: GainManaAbility) -> Self {
        let id = Self::new();

        db.mana_abilities.insert(
            id,
            GainManaAbilityInPlay {
                source,
                ability,
                temporary: false,
            },
        );

        id
    }

    pub(crate) fn upload_temporary_ability(
        db: &mut Database,
        source: CardId,
        ability: GainManaAbility,
    ) -> Self {
        let id = Self::new();

        db.mana_abilities.insert(
            id,
            GainManaAbilityInPlay {
                source,
                ability,
                temporary: true,
            },
        );

        id
    }
}
