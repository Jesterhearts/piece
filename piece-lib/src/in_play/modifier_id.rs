use std::{collections::HashSet, sync::atomic::Ordering};

use derive_more::{From, Into};
use indexmap::IndexMap;
use itertools::Itertools;
use tracing::Level;

use crate::{
    in_play::{
        ActivatedAbilityId, CardId, Database, GainManaAbilityId, StaticAbilityId, NEXT_MODIFIER_ID,
    },
    protogen::effects::BattlefieldModifier,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub(crate) struct ModifierId(usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifierInPlay {
    pub(crate) source: CardId,
    pub(crate) temporary: bool,
    pub(crate) active: bool,
    pub(crate) modifier: BattlefieldModifier,
    pub(crate) modifying: HashSet<CardId>,

    pub(crate) add_static_abilities: HashSet<StaticAbilityId>,
    pub(crate) add_activated_abilities: HashSet<ActivatedAbilityId>,
    pub(crate) add_mana_abilities: HashSet<GainManaAbilityId>,
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
            add_static_abilities.insert(StaticAbilityId::upload(
                db,
                source,
                ability.ability.as_ref().unwrap().clone(),
            ));
        }

        let mut add_activated_abilities = HashSet::default();
        if let Some(add) = modifier.modifier.add_ability.as_ref() {
            add_activated_abilities.insert(ActivatedAbilityId::upload(db, source, add.clone()));
        }

        let mut add_mana_abilities = HashSet::default();
        if let Some(add) = modifier.modifier.mana_ability.as_ref() {
            add_mana_abilities.insert(GainManaAbilityId::upload(db, source, add.clone()));
        }

        db.modifiers.insert(
            id,
            ModifierInPlay {
                source,
                temporary: true,
                active: false,
                modifier,
                modifying: Default::default(),
                add_static_abilities,
                add_activated_abilities,
                add_mana_abilities,
            },
        );

        id
    }

    #[instrument(level = Level::DEBUG, skip(modifiers))]
    pub(crate) fn activate(self, modifiers: &mut IndexMap<ModifierId, ModifierInPlay>) {
        let mut value = modifiers.shift_remove(&self).unwrap();
        value.active = true;
        modifiers.insert(self, value);
    }

    #[instrument(level = Level::DEBUG, skip(db))]
    pub(crate) fn deactivate(self, db: &mut Database) {
        debug!("modifier from {}", db[self].source.name(db));

        let modifier = db.modifiers.get_mut(&self).unwrap();
        modifier.active = false;
        let modifying = modifier.modifying.drain().collect_vec();

        if modifier.temporary {
            for id in modifier.add_activated_abilities.iter() {
                db.gc_abilities.push(*id);
            }

            for id in modifier.add_static_abilities.iter() {
                db.static_abilities.remove(id);
            }

            for id in modifier.add_mana_abilities.iter() {
                db.mana_abilities.remove(id);
            }

            db.modifiers.shift_remove(&self);
        }

        for card in modifying {
            card.apply_modifiers_layered(db);
        }
    }
}
