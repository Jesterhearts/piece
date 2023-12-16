use std::sync::atomic::Ordering;

use bevy_ecs::{component::Component, entity::Entity};
use derive_more::{Deref, DerefMut, From, Into};

use crate::{
    abilities::Ability,
    card::{
        ActivatedAbilityModifier, AddColors, AddPower, AddToughness, BasePowerModifier,
        BaseToughnessModifier, EtbAbilityModifier, Keyword, ModifyKeywords, RemoveAllColors,
        StaticAbilityModifier, TriggeredAbilityModifier,
    },
    controller::ControllerRestriction,
    effects::{
        effect_duration::{UntilEndOfTurn, UntilSourceLeavesBattlefield},
        BattlefieldModifier, DynamicPowerToughness, EffectDuration,
    },
    in_play::{
        AbilityId, Active, CardId, Database, EntireBattlefield, Global, Modifying, Temporary,
        NEXT_MODIFIER_SEQ,
    },
    targets::{Restriction, Restrictions},
    types::{AddSubtypes, AddTypes, RemoveAllSubtypes, RemoveSubtypes, RemoveTypes},
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord, Component)]
pub struct ModifierSeq(usize);

impl ModifierSeq {
    pub fn next() -> Self {
        Self(NEXT_MODIFIER_SEQ.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Default, Component, Deref, DerefMut)]
pub struct Modifiers(pub Vec<ModifierId>);

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Into)]
pub struct ModifierId(Entity);

impl ModifierId {
    pub fn remove_all_abilities(self, db: &mut Database) {
        db.modifiers
            .entity_mut(self.0)
            .insert(ActivatedAbilityModifier::RemoveAll)
            .insert(StaticAbilityModifier::RemoveAll)
            .insert(TriggeredAbilityModifier::RemoveAll)
            .insert(ModifyKeywords::Remove(Keyword::all()));
    }

    pub fn upload_temporary_modifier(
        db: &mut Database,
        cardid: CardId,
        modifier: &BattlefieldModifier,
    ) -> ModifierId {
        Self::upload_modifier(db, cardid, modifier, true)
    }

    pub fn upload_modifier(
        db: &mut Database,
        source: CardId,
        modifier: &BattlefieldModifier,
        temporary: bool,
    ) -> ModifierId {
        let mut entity = db.modifiers.spawn((
            modifier.controller,
            Restrictions(modifier.restrictions.clone()),
            source,
            Modifying::default(),
        ));

        if temporary {
            entity.insert(Temporary);
        }

        match modifier.duration {
            EffectDuration::UntilEndOfTurn => {
                entity.insert(UntilEndOfTurn);
            }
            EffectDuration::UntilSourceLeavesBattlefield => {
                entity.insert(UntilSourceLeavesBattlefield);
            }
        }

        if modifier.modifier.global {
            entity.insert(Global);
        }

        if modifier.modifier.entire_battlefield {
            entity.insert(EntireBattlefield);
        }

        if let Some(power) = modifier.modifier.base_power {
            entity.insert(BasePowerModifier(power));
        }

        if let Some(toughness) = modifier.modifier.base_toughness {
            entity.insert(BaseToughnessModifier(toughness));
        }

        if let Some(dynamic) = &modifier.modifier.dynamic_power_toughness {
            entity.insert(*dynamic);
        }

        if let Some(power) = modifier.modifier.add_power {
            entity.insert(AddPower(power));
        }

        if let Some(toughness) = modifier.modifier.add_toughness {
            entity.insert(AddToughness(toughness));
        }

        if !modifier.modifier.add_types.is_empty() {
            entity.insert(AddTypes(modifier.modifier.add_types.clone()));
        }

        if !modifier.modifier.add_subtypes.is_empty() {
            entity.insert(AddSubtypes(modifier.modifier.add_subtypes.clone()));
        }

        if !modifier.modifier.add_colors.is_empty() {
            entity.insert(AddColors(modifier.modifier.add_colors.clone()));
        }

        if !modifier.modifier.remove_types.is_empty() {
            entity.insert(RemoveTypes(modifier.modifier.remove_types.clone()));
        }

        if !modifier.modifier.remove_subtypes.is_empty() {
            entity.insert(RemoveSubtypes(modifier.modifier.remove_subtypes.clone()));
        }

        if modifier.modifier.remove_all_subtypes {
            entity.insert(RemoveAllSubtypes);
        }

        if modifier.modifier.remove_all_colors {
            entity.insert(RemoveAllColors);
        }

        if !modifier.modifier.remove_keywords.is_empty() {
            entity.insert(ModifyKeywords::Remove(
                modifier.modifier.remove_keywords.clone(),
            ));
        }

        if !modifier.modifier.add_keywords.is_empty() {
            entity.insert(ModifyKeywords::Add(modifier.modifier.add_keywords.clone()));
        }

        let modifierid = ModifierId(entity.id());

        if let Some(ability) = &modifier.modifier.add_ability {
            let id = AbilityId::upload_ability(db, source, Ability::Activated(ability.clone()));
            db.modifiers
                .entity_mut(modifierid.0)
                .insert(ActivatedAbilityModifier::Add(id));
        }

        if let Some(ability) = &modifier.modifier.mana_ability {
            let id = AbilityId::upload_ability(db, source, Ability::Mana(ability.clone()));
            db.modifiers
                .entity_mut(modifierid.0)
                .insert(ActivatedAbilityModifier::Add(id));
        }

        if modifier.modifier.remove_all_abilities {
            modifierid.remove_all_abilities(db);
        }

        modifierid
    }

    pub fn modifying(self, db: &mut Database) -> &Modifying {
        db.modifiers.get::<Modifying>(self.0).unwrap()
    }

    pub fn ability_modifier(self, db: &mut Database) -> Option<&ActivatedAbilityModifier> {
        db.modifiers.get::<ActivatedAbilityModifier>(self.0)
    }

    pub fn activate(self, db: &mut Database) {
        db.modifiers
            .entity_mut(self.0)
            .insert(Active)
            .insert(ModifierSeq(
                NEXT_MODIFIER_SEQ.fetch_add(1, Ordering::Relaxed),
            ));
    }

    pub fn deactivate(self, db: &mut Database) {
        let is_temporary = db.modifiers.get::<Temporary>(self.0).is_some();
        let modifying = self.modifying(db);

        if is_temporary && modifying.is_empty() {
            if let Some(ActivatedAbilityModifier::Add(ability)) = self.ability_modifier(db) {
                ability.delete(db);
            }

            db.modifiers.despawn(self.0);
        } else {
            db.modifiers.entity_mut(self.0).remove::<Active>();
        }
    }

    pub fn detach_all(&self, db: &mut Database) {
        db.modifiers.get_mut::<Modifying>(self.0).unwrap().clear();
        self.deactivate(db);
    }

    pub fn add_types(self, db: &mut Database) -> Option<&AddTypes> {
        db.modifiers.get::<AddTypes>(self.0)
    }

    pub fn add_subtypes(self, db: &mut Database) -> Option<&AddSubtypes> {
        db.modifiers.get::<AddSubtypes>(self.0)
    }

    pub fn remove_types(self, db: &mut Database) -> Option<&RemoveTypes> {
        db.modifiers.get::<RemoveTypes>(self.0)
    }

    pub fn remove_subtypes(self, db: &mut Database) -> Option<&RemoveSubtypes> {
        db.modifiers.get::<RemoveSubtypes>(self.0)
    }

    pub fn source(self, db: &mut Database) -> CardId {
        db.modifiers.get::<CardId>(self.0).copied().unwrap()
    }

    pub fn controller_restriction(self, db: &mut Database) -> ControllerRestriction {
        db.modifiers
            .get::<ControllerRestriction>(self.0)
            .copied()
            .unwrap()
    }

    pub fn restrictions(self, db: &mut Database) -> Vec<Restriction> {
        db.modifiers.get::<Restrictions>(self.0).cloned().unwrap().0
    }

    pub fn add_colors(self, db: &mut Database) -> Option<&AddColors> {
        db.modifiers.get::<AddColors>(self.0)
    }

    pub fn triggered_ability_modifiers(
        self,
        db: &mut Database,
    ) -> Option<&TriggeredAbilityModifier> {
        db.modifiers.get::<TriggeredAbilityModifier>(self.0)
    }

    pub fn etb_ability_modifiers(self, db: &mut Database) -> Option<&EtbAbilityModifier> {
        db.modifiers.get::<EtbAbilityModifier>(self.0)
    }

    pub fn static_ability_modifiers(self, db: &mut Database) -> Option<&StaticAbilityModifier> {
        db.modifiers.get::<StaticAbilityModifier>(self.0)
    }

    pub fn activated_ability_modifiers(
        self,
        db: &mut Database,
    ) -> Option<&ActivatedAbilityModifier> {
        db.modifiers.get::<ActivatedAbilityModifier>(self.0)
    }

    pub fn keyword_modifiers(self, db: &mut Database) -> Option<&ModifyKeywords> {
        db.modifiers.get::<ModifyKeywords>(self.0)
    }

    pub fn base_power(self, db: &mut Database) -> Option<i32> {
        db.modifiers.get::<BasePowerModifier>(self.0).map(|m| m.0)
    }

    pub fn base_toughness(self, db: &mut Database) -> Option<i32> {
        db.modifiers
            .get::<BaseToughnessModifier>(self.0)
            .map(|m| m.0)
    }

    pub fn add_power(self, db: &mut Database) -> Option<i32> {
        db.modifiers.get::<AddPower>(self.0).map(|a| a.0)
    }

    pub fn add_toughness(self, db: &mut Database) -> Option<i32> {
        db.modifiers.get::<AddToughness>(self.0).map(|a| a.0)
    }

    pub fn dynamic_power(self, db: &mut Database) -> Option<DynamicPowerToughness> {
        db.modifiers.get::<DynamicPowerToughness>(self.0).cloned()
    }

    pub fn remove_all_colors(self, db: &mut Database) -> bool {
        db.modifiers.get::<RemoveAllColors>(self.0).is_some()
    }
}