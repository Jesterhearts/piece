use uuid::Uuid;

use crate::{
    in_play::{CardId, Database, ModifierId},
    protogen::effects::static_ability,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub(crate) struct StaticAbilityId(Uuid);

#[derive(Debug)]
pub struct StaticAbilityInPlay {
    pub(crate) source: CardId,
    pub(crate) ability: static_ability::Ability,
    pub(crate) owned_modifier: Option<ModifierId>,
}

impl StaticAbilityId {
    pub(crate) fn new() -> Self {
        Self(Uuid::new_v4())
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
