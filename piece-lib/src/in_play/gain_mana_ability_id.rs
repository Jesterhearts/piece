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
    pub(crate) temporary: bool,
}

impl GainManaAbilityId {
    pub(crate) fn new() -> Self {
        Self(Uuid::new_v4())
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
