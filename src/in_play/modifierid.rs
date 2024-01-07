use std::{collections::HashSet, sync::atomic::Ordering};

use derive_more::{From, Into};
use indexmap::IndexMap;
use tracing::Level;

use crate::{
    effects::BattlefieldModifier,
    in_play::{CardId, NEXT_MODIFIER_ID},
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord)]
pub(crate) struct ModifierSeq(usize);

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub(crate) struct ModifierId(usize);

#[derive(Debug)]
pub struct ModifierInPlay {
    pub(crate) source: CardId,
    pub(crate) temporary: bool,
    pub(crate) active: bool,
    pub(crate) modifier: BattlefieldModifier,
    pub(crate) modifying: HashSet<CardId>,
}

impl ModifierId {
    pub(crate) fn new() -> Self {
        Self(NEXT_MODIFIER_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub(crate) fn upload_temporary_modifier(
        modifiers: &mut IndexMap<ModifierId, ModifierInPlay>,
        source: CardId,
        modifier: BattlefieldModifier,
    ) -> Self {
        let id = Self::new();
        modifiers.insert(
            id,
            ModifierInPlay {
                source,
                temporary: true,
                active: true,
                modifier,
                modifying: Default::default(),
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
