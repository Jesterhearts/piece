use std::sync::atomic::Ordering;

use derive_more::{From, Into};

use crate::{
    in_play::{CardId, Database, ModifierId, NEXT_ABILITY_ID},
    protogen::effects::static_ability,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub(crate) struct StaticAbilityId(usize);

#[derive(Debug)]
pub struct StaticAbilityInPlay {
    pub(crate) source: CardId,
    pub(crate) ability: static_ability::Ability,
    pub(crate) owned_modifier: Option<ModifierId>,
}

impl StaticAbilityId {
    pub(crate) fn new() -> Self {
        Self(NEXT_ABILITY_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub(crate) fn upload(
        db: &mut Database,
        source: CardId,
        ability: static_ability::Ability,
    ) -> Self {
        let id = Self::new();

        let owned_modifier =
            if let static_ability::Ability::BattlefieldModifier(modifier) = &ability {
                Some(ModifierId::upload_temporary_modifier(
                    db,
                    source,
                    modifier.clone(),
                ))
            } else {
                None
            };

        db.static_abilities.insert(
            id,
            StaticAbilityInPlay {
                source,
                ability,
                owned_modifier,
            },
        );

        id
    }
}
