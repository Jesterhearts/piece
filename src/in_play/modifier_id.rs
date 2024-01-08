use std::{collections::HashSet, sync::atomic::Ordering};

use derive_more::{From, Into};
use indexmap::IndexMap;
use tracing::Level;

use crate::{
    effects::BattlefieldModifier,
    in_play::{CardId, Database, StaticAbilityId, NEXT_MODIFIER_ID},
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub(crate) struct ModifierId(usize);

#[derive(Debug)]
pub struct ModifierInPlay {
    pub(crate) source: CardId,
    pub(crate) temporary: bool,
    pub(crate) active: bool,
    pub(crate) modifier: BattlefieldModifier,
    pub(crate) modifying: HashSet<CardId>,
    pub(crate) add_static_abilities: HashSet<StaticAbilityId>,
}

impl ModifierId {
    pub(crate) fn new() -> Self {
        Self(NEXT_MODIFIER_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub(crate) fn upload_temporary_modifier(
        db: &mut Database,
        source: CardId,
        modifier: BattlefieldModifier,
    ) -> Self {
        let id = Self::new();

        let mut add_static_abilities = HashSet::default();
        for ability in modifier.modifier.add_static_abilities.iter() {
            add_static_abilities.insert(StaticAbilityId::upload(db, source, ability.clone()));
        }

        db.modifiers.insert(
            id,
            ModifierInPlay {
                source,
                temporary: true,
                active: true,
                modifier,
                modifying: Default::default(),
                add_static_abilities,
            },
        );

        id
    }

    #[instrument(level = Level::DEBUG)]
    pub(crate) fn activate(self, modifiers: &mut IndexMap<ModifierId, ModifierInPlay>) {
        modifiers.get_mut(&self).unwrap().active = true;
    }

    #[instrument(level = Level::DEBUG)]
    pub(crate) fn deactivate(self, modifiers: &mut IndexMap<ModifierId, ModifierInPlay>) {
        let modifier = modifiers.get_mut(&self).unwrap();

        if modifier.temporary && modifier.modifying.is_empty() {
            modifiers.remove(&self);
        } else {
            modifier.active = false;
            modifier.modifying.clear();
        }
    }
}
