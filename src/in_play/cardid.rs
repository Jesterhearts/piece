use std::{
    collections::{HashMap, HashSet},
    sync::atomic::Ordering,
};

use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{With, Without},
};
use derive_more::From;
use indexmap::IndexSet;
use itertools::Itertools;
use strum::IntoEnumIterator;

use crate::{
    abilities::{
        Ability, ActivatedAbilities, ETBAbilities, GainMana, ModifiedActivatedAbilities,
        ModifiedETBAbilities, ModifiedStaticAbilities, ModifiedTriggers, StaticAbilities,
        StaticAbility, Triggers,
    },
    battlefield::{Battlefield, PendingResults},
    card::{
        keyword::SplitSecond, ActivatedAbilityModifier, AddPower, AddToughness, BackFace,
        BasePower, BaseToughness, CannotBeCountered, Card, Color, Colors, EtbAbilityModifier,
        EtbTapped, FrontFace, Keyword, Keywords, MarkedDamage, ModifiedBasePower,
        ModifiedBaseToughness, ModifiedColors, ModifiedKeywords, ModifyKeywords, Name, OracleText,
        PaidX, Revealed, StaticAbilityModifier, TargetIndividually, TriggeredAbilityModifier,
    },
    controller::ControllerRestriction,
    cost::{CastingCost, CostReducer, Ward},
    effects::{
        effect_duration::{self, UntilEndOfTurn, UntilSourceLeavesBattlefield},
        target_gains_counters::{counter, Counter},
        AnyEffect, DynamicPowerToughness, EffectDuration, Effects, Modes, ReplacementEffects,
        Token,
    },
    in_play::{
        self, cast_from, descend, exile_reason, life_gained_this_turn, times_descended_this_turn,
        AbilityId, Active, Attacking, AuraId, CastFrom, CounterId, Database, EntireBattlefield,
        ExileReason, ExiledWith, FaceDown, Global, InExile, InGraveyard, InHand, InLibrary,
        InStack, IsToken, LeftBattlefieldTurn, Manifested, ModifierId, ModifierSeq, Modifiers,
        Modifying, OnBattlefield, ReplacementEffectId, Tapped, Transformed, TriggerId, UniqueId,
        NEXT_BATTLEFIELD_SEQ, NEXT_GRAVEYARD_SEQ, NEXT_HAND_SEQ, NEXT_STACK_SEQ,
    },
    player::{
        mana_pool::{ManaSource, SourcedMana},
        Controller, Owner,
    },
    stack::{self, ActiveTarget, Settled, Stack, Targets},
    targets::{self, Cmc, Comparison, Dynamic, Restriction, Restrictions, SpellTarget},
    triggers::trigger_source,
    types::{ModifiedSubtypes, ModifiedTypes, Subtype, Subtypes, Type, Types},
    Cards,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Component)]
pub struct CardId(pub(super) Entity);

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Component)]
pub(crate) struct Cloning(pub(super) Entity);

impl From<Cloning> for CardId {
    fn from(value: Cloning) -> Self {
        Self(value.0)
    }
}

impl From<CardId> for Cloning {
    fn from(value: CardId) -> Self {
        Self(value.0)
    }
}

impl PartialEq<CardId> for Cloning {
    fn eq(&self, other: &CardId) -> bool {
        self.0 == other.0
    }
}

impl CardId {
    pub fn id(self, db: &Database) -> usize {
        db.get::<UniqueId>(self.0).unwrap().0
    }

    pub fn is_in_location<Location: Component + Ord>(self, db: &Database) -> bool {
        db.get::<Location>(self.0).is_some()
    }

    pub(crate) fn is_token(self, db: &Database) -> bool {
        db.get::<IsToken>(self.0).is_some()
    }

    pub(crate) fn facedown(self, db: &Database) -> bool {
        db.get::<FaceDown>(self.0).is_some()
    }

    pub(crate) fn transformed(self, db: &Database) -> bool {
        db.get::<Transformed>(self.0).is_some()
    }

    pub(crate) fn transform(self, db: &mut Database) {
        let front_face = self.faceup_face(db);
        let back_face = self.facedown_face(db);
        for counter in Counter::iter() {
            let count = CounterId::counters_on(db, front_face, counter);
            CounterId::add_counters(db, back_face, counter, count);
            CounterId::remove_counters(db, front_face, counter, count);
        }

        for modifier in front_face.modifiers(db) {
            back_face.apply_modifier(db, modifier);
        }

        front_face.remove_all_modifiers(db);

        let transformed = self.transformed(db);
        if transformed {
            db.entity_mut(front_face.0).remove::<Transformed>();
            db.entity_mut(back_face.0).remove::<Transformed>();
        } else {
            db.entity_mut(front_face.0).insert(Transformed);
            db.entity_mut(back_face.0).insert(Transformed);
        }
    }

    pub(crate) fn faceup_face(self, db: &Database) -> CardId {
        let transformed = self.transformed(db);
        if transformed {
            db.get::<BackFace>(self.0).map(|b| b.0).unwrap()
        } else {
            db.get::<FrontFace>(self.0).map(|f| f.0).unwrap_or(self)
        }
    }

    pub(crate) fn facedown_face(self, db: &Database) -> CardId {
        let transformed = self.transformed(db);
        if transformed {
            db.get::<FrontFace>(self.0).map(|f| f.0).unwrap()
        } else {
            db.get::<BackFace>(self.0).map(|b| b.0).unwrap()
        }
    }

    pub(crate) fn left_battlefield(self, db: &mut Database, turn_count: usize) {
        db.entity_mut(self.0)
            .insert(LeftBattlefieldTurn(turn_count));
    }

    pub(crate) fn left_battlefield_this_turn(db: &mut Database, turn_count: usize) -> Vec<CardId> {
        db.query::<(Entity, &LeftBattlefieldTurn)>()
            .iter(db)
            .filter_map(|(e, turn)| {
                if turn.0 == turn_count {
                    Some(Self(e))
                } else {
                    None
                }
            })
            .collect_vec()
    }

    pub fn move_to_hand(self, db: &mut Database) {
        if self.is_token(db) {
            self.move_to_limbo(db);
        } else {
            self.remove_all_modifiers(db);
            TriggerId::deactivate_all_for_card(db, self);
            ReplacementEffectId::deactivate_all_for_card(db, self);
            AbilityId::cleanup_temporary_abilities(db, self);
            self.deactivate_modifiers(db);

            self.untap(db);

            let owner = self.owner(db);
            *db.get_mut::<Controller>(self.0).unwrap() = owner.into();

            db.entity_mut(self.0)
                .remove::<InLibrary>()
                .remove::<InHand>()
                .remove::<InStack>()
                .remove::<OnBattlefield>()
                .remove::<InGraveyard>()
                .remove::<InExile>()
                .remove::<cast_from::Hand>()
                .remove::<cast_from::Exile>()
                .remove::<exile_reason::Cascade>()
                .remove::<effect_duration::UntilEndOfTurn>()
                .remove::<effect_duration::UntilSourceLeavesBattlefield>()
                .insert(InHand(NEXT_HAND_SEQ.fetch_add(1, Ordering::Relaxed)));
        }
    }

    pub(crate) fn move_to_stack(
        self,
        db: &mut Database,
        targets: Vec<Vec<ActiveTarget>>,
        from: Option<CastFrom>,
        chosen_modes: Vec<usize>,
    ) {
        if Stack::split_second(db) {
            return;
        }

        if self.is_token(db) {
            self.move_to_limbo(db);
        } else {
            self.remove_all_modifiers(db);
            TriggerId::deactivate_all_for_card(db, self);
            ReplacementEffectId::deactivate_all_for_card(db, self);
            AbilityId::cleanup_temporary_abilities(db, self);
            self.deactivate_modifiers(db);

            self.untap(db);

            let owner = self.owner(db);
            *db.get_mut::<Controller>(self.0).unwrap() = owner.into();

            let mut entity = db.entity_mut(self.0);
            entity
                .remove::<InLibrary>()
                .remove::<InHand>()
                .remove::<InStack>()
                .remove::<OnBattlefield>()
                .remove::<InGraveyard>()
                .remove::<InExile>()
                .remove::<cast_from::Hand>()
                .remove::<cast_from::Exile>()
                .remove::<exile_reason::Cascade>()
                .remove::<effect_duration::UntilEndOfTurn>()
                .remove::<effect_duration::UntilSourceLeavesBattlefield>()
                .insert(InStack(NEXT_STACK_SEQ.fetch_add(1, Ordering::Relaxed)))
                .insert(Targets(targets));

            if !chosen_modes.is_empty() {
                debug!("Chosen modes: {:?}", chosen_modes);
                entity.insert(stack::Modes(chosen_modes));
            }

            if let Some(from) = from {
                match from {
                    CastFrom::Hand => {
                        entity.insert(cast_from::Hand);
                    }
                    CastFrom::Exile => {
                        entity.insert(cast_from::Exile);
                    }
                }
            }
        }
    }

    pub(crate) fn cast_from_hand(self, db: &Database) -> bool {
        db.get::<InStack>(self.0).is_some() && db.get::<cast_from::Hand>(self.0).is_some()
    }

    pub(crate) fn move_to_battlefield(self, db: &mut Database) {
        db.cards
            .entity_mut(self.0)
            .remove::<InLibrary>()
            .remove::<InHand>()
            .remove::<InStack>()
            .remove::<OnBattlefield>()
            .remove::<InGraveyard>()
            .remove::<InExile>()
            .remove::<cast_from::Hand>()
            .remove::<cast_from::Exile>()
            .remove::<exile_reason::Cascade>()
            .remove::<effect_duration::UntilEndOfTurn>()
            .remove::<effect_duration::UntilSourceLeavesBattlefield>()
            .remove::<LeftBattlefieldTurn>()
            .insert(OnBattlefield(
                NEXT_BATTLEFIELD_SEQ.fetch_add(1, Ordering::Relaxed),
            ));

        TriggerId::activate_all_for_card(db, self);
        ReplacementEffectId::activate_all_for_card(db, self);
    }

    pub(crate) fn move_to_graveyard(self, db: &mut Database) {
        if self.is_token(db) {
            self.move_to_limbo(db);
        } else {
            descend(db, self.owner(db));

            self.remove_all_modifiers(db);
            TriggerId::deactivate_all_for_card(db, self);
            ReplacementEffectId::deactivate_all_for_card(db, self);
            AbilityId::cleanup_temporary_abilities(db, self);
            self.deactivate_modifiers(db);

            self.untap(db);

            let owner = self.owner(db);
            *db.get_mut::<Controller>(self.0).unwrap() = owner.into();

            db.entity_mut(self.0)
                .remove::<InLibrary>()
                .remove::<InHand>()
                .remove::<InStack>()
                .remove::<OnBattlefield>()
                .remove::<InGraveyard>()
                .remove::<InExile>()
                .remove::<cast_from::Hand>()
                .remove::<cast_from::Exile>()
                .remove::<exile_reason::Cascade>()
                .remove::<effect_duration::UntilEndOfTurn>()
                .remove::<effect_duration::UntilSourceLeavesBattlefield>()
                .insert(InGraveyard(
                    NEXT_GRAVEYARD_SEQ.fetch_add(1, Ordering::Relaxed),
                ));
        }
    }

    pub(crate) fn move_to_library(self, db: &mut Database) -> bool {
        if self.is_token(db) {
            self.move_to_limbo(db);
            false
        } else {
            self.remove_all_modifiers(db);
            TriggerId::deactivate_all_for_card(db, self);
            ReplacementEffectId::deactivate_all_for_card(db, self);
            AbilityId::cleanup_temporary_abilities(db, self);
            self.deactivate_modifiers(db);

            self.untap(db);

            let owner = self.owner(db);
            *db.get_mut::<Controller>(self.0).unwrap() = owner.into();

            db.entity_mut(self.0)
                .remove::<InLibrary>()
                .remove::<InHand>()
                .remove::<InStack>()
                .remove::<OnBattlefield>()
                .remove::<InGraveyard>()
                .remove::<InExile>()
                .remove::<cast_from::Hand>()
                .remove::<cast_from::Exile>()
                .remove::<exile_reason::Cascade>()
                .remove::<effect_duration::UntilEndOfTurn>()
                .remove::<effect_duration::UntilSourceLeavesBattlefield>()
                .insert(InLibrary);
            true
        }
    }

    pub(crate) fn move_to_exile(
        self,
        db: &mut Database,
        source: CardId,
        reason: Option<ExileReason>,
        duration: EffectDuration,
    ) {
        if self.is_token(db) {
            self.move_to_limbo(db);
        } else {
            self.remove_all_modifiers(db);
            TriggerId::deactivate_all_for_card(db, self);
            ReplacementEffectId::deactivate_all_for_card(db, self);
            AbilityId::cleanup_temporary_abilities(db, self);
            self.deactivate_modifiers(db);

            self.untap(db);

            let owner = self.owner(db);
            *db.get_mut::<Controller>(self.0).unwrap() = owner.into();

            let mut entity = db.entity_mut(self.0);
            entity
                .remove::<InLibrary>()
                .remove::<InHand>()
                .remove::<InStack>()
                .remove::<OnBattlefield>()
                .remove::<InGraveyard>()
                .remove::<InExile>()
                .remove::<cast_from::Hand>()
                .remove::<cast_from::Exile>()
                .remove::<exile_reason::Cascade>()
                .remove::<effect_duration::UntilEndOfTurn>()
                .remove::<effect_duration::UntilSourceLeavesBattlefield>()
                .insert(InExile)
                .insert(ExiledWith(source));

            match duration {
                EffectDuration::Permanently => {}
                EffectDuration::UntilEndOfTurn => {
                    entity.insert(effect_duration::UntilEndOfTurn);
                }
                EffectDuration::UntilSourceLeavesBattlefield => {
                    entity.insert(effect_duration::UntilSourceLeavesBattlefield);
                }
                EffectDuration::UntilTargetLeavesBattlefield => {
                    unreachable!()
                }
            }

            if let Some(reason) = reason {
                match reason {
                    ExileReason::Cascade => {
                        entity.insert(exile_reason::Cascade);
                    }
                    ExileReason::Craft => {
                        entity.insert(exile_reason::Craft);
                    }
                }
            }
        }
    }

    pub(crate) fn move_to_limbo(self, db: &mut Database) {
        self.remove_all_modifiers(db);
        TriggerId::deactivate_all_for_card(db, self);
        ReplacementEffectId::deactivate_all_for_card(db, self);
        AbilityId::cleanup_temporary_abilities(db, self);
        self.deactivate_modifiers(db);

        self.untap(db);

        let owner = self.owner(db);
        *db.get_mut::<Controller>(self.0).unwrap() = owner.into();

        let mut entity = db.entity_mut(self.0);
        entity
            .remove::<InLibrary>()
            .remove::<InHand>()
            .remove::<InStack>()
            .remove::<OnBattlefield>()
            .remove::<InGraveyard>()
            .remove::<InExile>()
            .remove::<cast_from::Hand>()
            .remove::<cast_from::Exile>()
            .remove::<exile_reason::Cascade>()
            .remove::<effect_duration::UntilEndOfTurn>()
            .remove::<effect_duration::UntilSourceLeavesBattlefield>();
    }

    pub(crate) fn cleanup_tokens_in_limbo(db: &mut Database) {
        for entity in db
            .query_filtered::<Entity, (With<IsToken>, Without<OnBattlefield>)>()
            .iter(db)
            .collect_vec()
        {
            db.despawn(entity);
        }
    }

    pub(crate) fn remove_all_modifiers(self, db: &mut Database) {
        for mut modifying in db
            .modifiers
            .query::<&mut Modifying>()
            .iter_mut(&mut db.modifiers)
        {
            modifying.remove(&self);
        }
    }

    pub(crate) fn modifiers(self, db: &mut Database) -> Vec<ModifierId> {
        db.modifiers
            .query::<(Entity, &Modifying)>()
            .iter(&db.modifiers)
            .filter_map(|(entity, modifying)| {
                if modifying.contains(&self) {
                    Some(ModifierId::from(entity))
                } else {
                    None
                }
            })
            .collect()
    }

    pub(crate) fn deactivate_modifiers(self, db: &mut Database) {
        let mut entities = vec![];

        for (entity, source, mut modifying) in db.modifiers.query_filtered::<(Entity, &CardId, &mut Modifying), With<UntilSourceLeavesBattlefield>>().iter_mut(&mut db.modifiers) {
            if *source == self {
                modifying.clear();
                entities.push(entity);
            }
        }

        for entity in entities {
            ModifierId::from(entity).deactivate(db);
        }
    }

    pub(crate) fn activate_modifiers(self, db: &mut Database) {
        let mut entities = vec![];

        for (entity, source) in db
            .modifiers
            .query::<(Entity, &CardId)>()
            .iter_mut(&mut db.modifiers)
        {
            if *source == self {
                entities.push(entity);
            }
        }

        for entity in entities {
            ModifierId::from(entity).activate(db);
        }
    }

    pub(crate) fn apply_modifiers_layered(self, db: &mut Database) {
        TriggerId::unsubscribe_all_for_card(db, self);

        let on_battlefield = Self::is_in_location::<OnBattlefield>(self, db);

        let modifiers = db
            .modifiers
            .query_filtered::<(
                Entity,
                &ModifierSeq,
                &Modifying,
                Option<&Global>,
                Option<&EntireBattlefield>,
            ), With<Active>>()
            .iter(&db.modifiers)
            .sorted_by_key(|(_, seq, _, _, _)| *seq)
            .filter_map(|(entity, _, modifying, global, entire_battlefield)| {
                if global.is_some()
                    || (on_battlefield && entire_battlefield.is_some())
                    || modifying.contains(&self)
                {
                    Some(ModifierId::from(entity))
                } else {
                    None
                }
            })
            .collect_vec();

        let facedown = self.facedown(db) && !self.transformed(db);
        let source = self.faceup_face(db);

        let mut base_power = if facedown {
            Some(2)
        } else {
            db.get::<BasePower>(source.0).map(|bp| bp.0)
        };
        let mut base_toughness = if facedown {
            Some(2)
        } else {
            db.get::<BaseToughness>(source.0).map(|bt| bt.0)
        };
        let mut types = if facedown {
            IndexSet::from([Type::Creature])
        } else {
            db.get::<Types>(source.0).unwrap().0.clone()
        };
        let mut subtypes = if facedown {
            Default::default()
        } else {
            db.get::<Subtypes>(source.0).unwrap().0.clone()
        };
        let mut keywords = if facedown {
            ::counter::Counter::default()
        } else {
            db.get::<Keywords>(source.0).unwrap().0.clone()
        };
        let mut colors: HashSet<Color> = if facedown {
            HashSet::default()
        } else {
            db.get::<Colors>(source.0)
                .unwrap()
                .0
                .union(&db.get::<CastingCost>(source.0).unwrap().colors())
                .copied()
                .collect()
        };
        let mut triggers = if facedown {
            vec![]
        } else {
            db.get::<Triggers>(source.0)
                .cloned()
                .map(|t| t.0)
                .unwrap_or_default()
        };
        let mut etb_abilities = if facedown {
            vec![]
        } else {
            db.get::<ETBAbilities>(source.0)
                .cloned()
                .map(|t| t.0)
                .unwrap_or_default()
        };
        let mut static_abilities = if facedown {
            vec![]
        } else {
            db.get::<StaticAbilities>(source.0)
                .cloned()
                .map(|s| s.0)
                .unwrap_or_default()
        };
        let mut activated_abilities = if facedown {
            vec![]
        } else {
            db.get::<ActivatedAbilities>(source.0)
                .cloned()
                .map(|s| s.0)
                .unwrap_or_default()
        };

        let mut applied_modifiers: HashSet<ModifierId> = Default::default();

        // TODO control changing effects go here
        // TODO text changing effects go here

        for modifier in modifiers.iter().copied() {
            if !applied_modifiers.contains(&modifier) {
                let source = modifier.source(db);
                let controller_restriction = modifier.controller_restriction(db);
                let restrictions = modifier.restrictions(db);
                if !self.passes_restrictions_given_attributes(
                    db,
                    source,
                    self.controller(db),
                    controller_restriction,
                    &restrictions,
                    &types,
                    &subtypes,
                    &keywords,
                    &colors,
                    base_toughness,
                ) {
                    continue;
                }
            }

            if let Some(add) = modifier.add_types(db) {
                applied_modifiers.insert(modifier);
                types.extend(add.0.iter().copied())
            }

            if let Some(add) = modifier.add_subtypes(db) {
                applied_modifiers.insert(modifier);
                subtypes.extend(add.0.iter().copied())
            }

            if let Some(remove) = modifier.remove_types(db) {
                applied_modifiers.insert(modifier);
                for ty in remove.iter() {
                    types.remove(ty);
                }
            }

            if let Some(remove) = modifier.remove_subtypes(db) {
                applied_modifiers.insert(modifier);
                for ty in remove.iter() {
                    subtypes.remove(ty);
                }
            }
        }

        for (ty, add_ability) in AbilityId::land_abilities() {
            if subtypes.contains(&ty) {
                activated_abilities.push(add_ability(db, self));
            }
        }

        for modifier in modifiers.iter().copied() {
            if !applied_modifiers.contains(&modifier) {
                let source = modifier.source(db);
                let controller_restriction = modifier.controller_restriction(db);
                let restrictions = modifier.restrictions(db);
                if !self.passes_restrictions_given_attributes(
                    db,
                    source,
                    self.controller(db),
                    controller_restriction,
                    &restrictions,
                    &types,
                    &subtypes,
                    &keywords,
                    &colors,
                    base_toughness,
                ) {
                    continue;
                }
            }

            if let Some(add) = modifier.add_colors(db) {
                applied_modifiers.insert(modifier);
                colors.extend(add.0.iter().copied())
            }

            if modifier.remove_all_colors(db) {
                applied_modifiers.insert(modifier);
                colors.clear();
            }
        }

        if colors.len() != 1 {
            colors.remove(&Color::Colorless);
        }

        for modifier in modifiers.iter().copied() {
            if !applied_modifiers.contains(&modifier) {
                let source = modifier.source(db);
                let controller_restriction = modifier.controller_restriction(db);
                let restrictions = modifier.restrictions(db);
                if !self.passes_restrictions_given_attributes(
                    db,
                    source,
                    self.controller(db),
                    controller_restriction,
                    &restrictions,
                    &types,
                    &subtypes,
                    &keywords,
                    &colors,
                    base_toughness,
                ) {
                    continue;
                }
            }

            if let Some(modify) = modifier.triggered_ability_modifiers(db) {
                applied_modifiers.insert(modifier);
                match modify {
                    TriggeredAbilityModifier::RemoveAll => triggers.clear(),
                    TriggeredAbilityModifier::Add(add) => triggers.push(*add),
                }
            }

            if let Some(modify) = modifier.etb_ability_modifiers(db) {
                applied_modifiers.insert(modifier);
                match modify {
                    EtbAbilityModifier::RemoveAll => etb_abilities.clear(),
                    EtbAbilityModifier::Add(add) => etb_abilities.push(*add),
                }
            }

            if let Some(modify) = modifier.static_ability_modifiers(db) {
                applied_modifiers.insert(modifier);
                match modify {
                    StaticAbilityModifier::AddAll(add) => {
                        for ability in add.iter() {
                            static_abilities.push(ability.clone());
                        }
                    }
                    StaticAbilityModifier::RemoveAll => static_abilities.clear(),
                }
            }

            if let Some(modify) = modifier.activated_ability_modifiers(db) {
                applied_modifiers.insert(modifier);
                match modify {
                    ActivatedAbilityModifier::Add(add) => activated_abilities.push(*add),
                    ActivatedAbilityModifier::RemoveAll => activated_abilities.clear(),
                }
            }

            if let Some(modify) = modifier.keyword_modifiers(db) {
                match modify {
                    ModifyKeywords::Remove(remove) => keywords.retain(|kw, _| !remove.contains(kw)),
                    ModifyKeywords::Add(add) => keywords.extend(add.iter()),
                }
            }
        }

        let mut add_power = 0;
        let mut add_toughness = 0;

        for modifier in modifiers.iter().copied() {
            if !applied_modifiers.contains(&modifier) {
                let source = modifier.source(db);
                let controller_restriction = modifier.controller_restriction(db);
                let restrictions = modifier.restrictions(db);
                if !self.passes_restrictions_given_attributes(
                    db,
                    source,
                    self.controller(db),
                    controller_restriction,
                    &restrictions,
                    &types,
                    &subtypes,
                    &keywords,
                    &colors,
                    base_toughness.map(|t| t + add_toughness),
                ) {
                    continue;
                }
            }

            if let Some(base) = modifier.base_power(db) {
                base_power = Some(base);
            }

            if let Some(base) = modifier.base_toughness(db) {
                base_toughness = Some(base);
            }

            add_power += modifier.add_power(db).unwrap_or_default();
            add_toughness += modifier.add_toughness(db).unwrap_or_default();

            if let Some(dynamic) = modifier.dynamic_power(db) {
                match dynamic {
                    DynamicPowerToughness::NumberOfCountersOnThis(counter) => {
                        let source = modifier.source(db);
                        let to_add = CounterId::counters_on(db, source, counter);
                        add_power += to_add as i32;
                        add_toughness += to_add as i32;
                    }
                }
            }
        }

        let p1p1 = CounterId::counters_of_type_on::<counter::P1P1>(db, source);
        add_power += p1p1 as i32;
        add_toughness += p1p1 as i32;

        let m1m1 = CounterId::counters_of_type_on::<counter::M1M1>(db, source);
        add_power -= m1m1 as i32;
        add_toughness -= m1m1 as i32;

        if let Some(bp) = base_power {
            db.entity_mut(source.0).insert(ModifiedBasePower(bp));
        }
        if let Some(bt) = base_toughness {
            db.entity_mut(source.0).insert(ModifiedBaseToughness(bt));
        }

        for trigger in triggers.iter() {
            trigger.add_listener(db, source);
        }

        db.entity_mut(source.0)
            .insert(AddPower(add_power))
            .insert(AddToughness(add_toughness))
            .insert(ModifiedTypes(types))
            .insert(ModifiedColors(colors))
            .insert(ModifiedKeywords(keywords))
            .insert(ModifiedSubtypes(subtypes))
            .insert(ModifiedTriggers(triggers))
            .insert(ModifiedETBAbilities(etb_abilities))
            .insert(ModifiedStaticAbilities(static_abilities))
            .insert(ModifiedActivatedAbilities(activated_abilities));
    }

    pub(crate) fn triggers(self, db: &Database) -> Vec<TriggerId> {
        db.get::<ModifiedTriggers>(self.0)
            .map(|t| t.0.clone())
            .or_else(|| db.get::<Triggers>(self.0).map(|t| t.0.clone()))
            .unwrap_or_default()
    }

    pub fn equal(self, db: &Database, other: CardId) -> bool {
        self.name(db) == other.name(db)
            && self.power(db) == other.power(db)
            && self.toughness(db) == other.toughness(db)
            && self.types(db) == other.types(db)
            && self.colors(db) == other.colors(db)
            && self.keywords(db) == other.keywords(db)
            && self.activated_abilities(db) == other.activated_abilities(db)
            && self.tapped(db) == other.tapped(db)
    }

    pub(crate) fn etb_abilities(self, db: &Database) -> Vec<AbilityId> {
        db.get::<ModifiedETBAbilities>(self.0)
            .cloned()
            .map(|m| m.0)
            .or_else(|| db.get::<ETBAbilities>(self.0).cloned().map(|m| m.0))
            .unwrap_or_default()
    }

    pub(crate) fn static_abilities(self, db: &Database) -> Vec<StaticAbility> {
        db.get::<ModifiedStaticAbilities>(self.0)
            .cloned()
            .map(|m| m.0)
            .or_else(|| db.get::<StaticAbilities>(self.0).cloned().map(|m| m.0))
            .unwrap_or_default()
    }
    pub fn activated_abilities(self, db: &Database) -> Vec<AbilityId> {
        db.get::<ModifiedActivatedAbilities>(self.0)
            .cloned()
            .map(|m| m.0)
            .or_else(|| db.get::<ActivatedAbilities>(self.0).cloned().map(|m| m.0))
            .unwrap_or_default()
    }

    pub(crate) fn controller(self, db: &Database) -> Controller {
        db.get::<Controller>(self.0).copied().unwrap()
    }

    pub(crate) fn owner(self, db: &Database) -> Owner {
        db.get::<Owner>(self.0).copied().unwrap()
    }

    pub(crate) fn etb_tapped(self, db: &Database) -> bool {
        db.get::<EtbTapped>(self.0).is_some()
    }

    pub(crate) fn apply_modifier(self, db: &mut Database, modifier: ModifierId) {
        db.modifiers
            .get_mut::<Modifying>(modifier.into())
            .unwrap()
            .insert(self);
        modifier.activate(db);
        self.apply_modifiers_layered(db);
    }

    pub(crate) fn effects(self, db: &Database) -> Vec<AnyEffect> {
        db.get::<Effects>(self.0).cloned().unwrap_or_default().0
    }

    pub(crate) fn modes(self, db: &Database) -> Option<Modes> {
        db.get::<Modes>(self.0).cloned()
    }

    pub(crate) fn has_modes(&self, db: &mut Database) -> bool {
        db.get::<Modes>(self.0).is_some()
    }

    #[allow(unused)]
    pub(crate) fn needs_targets(self, db: &mut Database) -> Vec<usize> {
        let effects = self.effects(db);
        let controller = self.controller(db);
        let aura_targets = self.aura(db).map(|_| 1);
        std::iter::once(())
            .filter_map(|()| aura_targets)
            .chain(
                effects
                    .into_iter()
                    .map(|effect| effect.into_effect(db, controller))
                    .map(|effect| effect.needs_targets()),
            )
            .collect_vec()
    }

    pub(crate) fn wants_targets(&self, db: &mut Database) -> Vec<usize> {
        let effects = self.effects(db);
        let controller = self.controller(db);
        let aura_targets = self.aura(db).map(|_| 1);
        std::iter::once(())
            .filter_map(|()| aura_targets)
            .chain(
                effects
                    .into_iter()
                    .map(|effect| effect.into_effect(db, controller))
                    .map(|effect| effect.wants_targets()),
            )
            .collect_vec()
    }

    pub(crate) fn restrictions(self, db: &Database) -> Vec<Restriction> {
        db.get::<Restrictions>(self.0)
            .cloned()
            .map(|r| r.0)
            .unwrap()
    }

    pub(crate) fn passes_restrictions(
        self,
        db: &mut Database,
        source: CardId,
        controller_restriction: ControllerRestriction,
        restrictions: &[Restriction],
    ) -> bool {
        self.passes_restrictions_given_attributes(
            db,
            source,
            self.controller(db),
            controller_restriction,
            restrictions,
            &self.types(db),
            &self.subtypes(db),
            &self.keywords(db),
            &self.colors(db),
            self.toughness(db),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn passes_restrictions_given_attributes(
        self,
        db: &mut Database,
        source: CardId,
        self_controller: Controller,
        controller_restriction: ControllerRestriction,
        restrictions: &[Restriction],
        self_types: &IndexSet<Type>,
        self_subtypes: &IndexSet<Subtype>,
        self_keywords: &::counter::Counter<Keyword>,
        self_colors: &HashSet<Color>,
        self_toughness: Option<i32>,
    ) -> bool {
        match controller_restriction {
            ControllerRestriction::Any => {}
            ControllerRestriction::You => {
                if source.controller(db) != self_controller {
                    return false;
                }
            }
            ControllerRestriction::Opponent => {
                if source.controller(db) == self_controller {
                    return false;
                }
            }
        }

        for restriction in restrictions.iter() {
            match restriction {
                Restriction::NotSelf => {
                    if source == self {
                        return false;
                    }
                }
                Restriction::Self_ => {
                    if source != self {
                        return false;
                    }
                }
                Restriction::OfType { types, subtypes } => {
                    if !types.is_empty() && self_types.is_disjoint(types) {
                        return false;
                    }

                    if !subtypes.is_empty() && self_subtypes.is_disjoint(subtypes) {
                        return false;
                    }
                }
                Restriction::NotOfType { types, subtypes } => {
                    if !types.is_empty() && !self_types.is_disjoint(types) {
                        return false;
                    }
                    if !subtypes.is_empty() && !self_subtypes.is_disjoint(subtypes) {
                        return false;
                    }
                }
                Restriction::Toughness(comparison) => {
                    if self_toughness.is_none() {
                        return false;
                    }

                    let toughness = self_toughness.unwrap();
                    if !match comparison {
                        Comparison::LessThan(target) => toughness < *target,
                        Comparison::LessThanOrEqual(target) => toughness <= *target,
                        Comparison::GreaterThan(target) => toughness > *target,
                        Comparison::GreaterThanOrEqual(target) => toughness >= *target,
                    } {
                        return false;
                    }
                }
                Restriction::ControllerControlsBlackOrGreen => {
                    let colors = Battlefield::controlled_colors(db, self_controller);
                    if !(colors.contains(&Color::Green) || colors.contains(&Color::Black)) {
                        return false;
                    }
                }
                Restriction::ControllerHandEmpty => {
                    if self_controller.has_cards::<InHand>(db) {
                        return false;
                    }
                }
                Restriction::OfColor(ofcolors) => {
                    if self_colors.is_disjoint(ofcolors) {
                        return false;
                    }
                }
                Restriction::Cmc(cmc_test) => {
                    let cmc = self.cost(db).cmc() as i32;
                    match cmc_test {
                        Cmc::Comparison(comparison) => {
                            let matches = match comparison {
                                Comparison::LessThan(i) => cmc < *i,
                                Comparison::LessThanOrEqual(i) => cmc <= *i,
                                Comparison::GreaterThan(i) => cmc > *i,
                                Comparison::GreaterThanOrEqual(i) => cmc >= *i,
                            };
                            if !matches {
                                return false;
                            }
                        }
                        Cmc::Dynamic(dy) => match dy {
                            Dynamic::X => {
                                debug!("Destroying cmc {} vs x of {}", cmc, source.get_x(db));
                                if source.get_x(db) as i32 != cmc {
                                    return false;
                                }
                            }
                        },
                    }
                }
                Restriction::InGraveyard => {
                    if !self.is_in_location::<InGraveyard>(db) {
                        return false;
                    }
                }
                Restriction::OnBattlefield => {
                    if !self.is_in_location::<OnBattlefield>(db) {
                        return false;
                    }
                }
                Restriction::CastFromHand => {
                    if !self.cast_from_hand(db) {
                        return false;
                    }
                }
                Restriction::AttackingOrBlocking => {
                    // TODO blocking
                    if !self.attacking(db) {
                        return false;
                    }
                }
                Restriction::InLocation { locations } => {
                    if !locations.iter().any(|loc| match loc {
                        targets::Location::Battlefield => self.is_in_location::<OnBattlefield>(db),
                        targets::Location::Graveyard => self.is_in_location::<InGraveyard>(db),
                        targets::Location::Exile => self.is_in_location::<InExile>(db),
                        targets::Location::Library => self.is_in_location::<InLibrary>(db),
                        targets::Location::Hand => self.is_in_location::<InHand>(db),
                        targets::Location::Stack => self.is_in_location::<InStack>(db),
                    }) {
                        return false;
                    }
                }
                Restriction::Attacking => {
                    if !self.attacking(db) {
                        return false;
                    }
                }
                Restriction::NotKeywords(not_keywords) => {
                    if self_keywords
                        .keys()
                        .any(|keyword| not_keywords.contains(keyword))
                    {
                        return false;
                    }
                }
                Restriction::LifeGainedThisTurn(count) => {
                    let gained_this_turn = life_gained_this_turn(db, self_controller.into());
                    if gained_this_turn < *count {
                        return false;
                    }
                }
                Restriction::DescendedThisTurn => {
                    let descended = times_descended_this_turn(db, self.controller(db).into());
                    if descended < 1 {
                        return false;
                    }
                }
                Restriction::Tapped => {
                    if !self.tapped(db) {
                        return false;
                    }
                }
            }
        }

        true
    }

    pub(crate) fn apply_aura(self, db: &mut Database, aura: AuraId) {
        let modifiers = aura.modifiers(db);

        for modifier in modifiers.iter() {
            self.apply_modifier(db, *modifier);
        }
    }

    pub(crate) fn marked_damage(self, db: &Database) -> i32 {
        db.get::<MarkedDamage>(self.0)
            .copied()
            .unwrap_or_default()
            .0
    }

    pub(crate) fn mark_damage(self, db: &mut Database, amount: usize) {
        if let Some(mut marked) = db.get_mut::<MarkedDamage>(self.0) {
            **marked += amount as i32;
        } else {
            db.entity_mut(self.0).insert(MarkedDamage(amount as i32));
        }
    }

    pub(crate) fn clear_damage(self, db: &mut Database) {
        if let Some(mut marked) = db.get_mut::<MarkedDamage>(self.0) {
            **marked = 0;
        }
    }

    pub(crate) fn power(self, db: &Database) -> Option<i32> {
        db.get::<ModifiedBasePower>(self.0)
            .map(|bp| bp.0)
            .or_else(|| db.get::<BasePower>(self.0).map(|bp| bp.0))
            .map(|bp| bp + db.get::<AddPower>(self.0).map(|a| a.0).unwrap_or_default())
    }

    pub(crate) fn toughness(self, db: &Database) -> Option<i32> {
        db.get::<ModifiedBaseToughness>(self.0)
            .map(|bp| bp.0)
            .or_else(|| db.get::<BaseToughness>(self.0).map(|bt| bt.0))
            .map(|bp| {
                bp + db
                    .get::<AddToughness>(self.0)
                    .map(|a| a.0)
                    .unwrap_or_default()
            })
    }

    pub(crate) fn aura(self, db: &Database) -> Option<AuraId> {
        db.get::<AuraId>(self.0).copied()
    }

    pub(crate) fn colors(self, db: &Database) -> HashSet<Color> {
        db.get::<ModifiedColors>(self.0)
            .map(|bp| bp.0.clone())
            .or_else(|| {
                db.get::<Colors>(self.0).map(|c| {
                    let mut colors = db.get::<CastingCost>(self.0).unwrap().colors();
                    colors.extend(c.iter());
                    if colors.len() != 1 {
                        colors.remove(&Color::Colorless);
                    }
                    colors
                })
            })
            .unwrap_or_default()
    }

    pub fn color_identity(self, db: &mut Database) -> HashSet<Color> {
        let mut identity = self.colors(db);

        let abilities = db.get::<ActivatedAbilities>(self.0).cloned().unwrap();

        for ability in abilities.iter() {
            let ability = ability.ability(db);
            if let Some(cost) = ability.cost() {
                for mana in cost.mana_cost.iter() {
                    let color = mana.color();
                    identity.insert(color);
                }
            }

            if let Ability::Mana(mana) = ability {
                match mana.gain {
                    GainMana::Specific { gains } => {
                        for gain in gains.iter() {
                            identity.insert(gain.color());
                        }
                    }
                    GainMana::Choice { choices } => {
                        for choice in choices.iter() {
                            for mana in choice.iter() {
                                identity.insert(mana.color());
                            }
                        }
                    }
                }
            }
        }

        identity
    }

    pub(crate) fn types_intersect(self, db: &Database, types: &IndexSet<Type>) -> bool {
        types.is_empty() || !self.types(db).is_disjoint(types)
    }

    pub(crate) fn types(self, db: &Database) -> IndexSet<Type> {
        db.get::<ModifiedTypes>(self.0)
            .map(|t| t.0.clone())
            .or_else(|| db.get::<Types>(self.0).map(|t| t.0.clone()))
            .unwrap_or_default()
    }

    pub(crate) fn subtypes_intersect(self, db: &Database, types: &IndexSet<Subtype>) -> bool {
        types.is_empty() || !self.subtypes(db).is_disjoint(types)
    }

    pub(crate) fn subtypes(self, db: &Database) -> IndexSet<Subtype> {
        db.get::<ModifiedSubtypes>(self.0)
            .map(|t| t.0.clone())
            .or_else(|| db.get::<Subtypes>(self.0).map(|t| t.0.clone()))
            .unwrap_or_default()
    }

    #[must_use]
    pub fn upload(db: &mut Database, cards: &Cards, player: Owner, name: &str) -> CardId {
        let card = cards.get(name).expect("Valid card name");

        Self::upload_card(db, card, player, InLibrary, false)
    }

    #[must_use]
    pub(crate) fn upload_token(db: &mut Database, player: Owner, token: Token) -> CardId {
        let card: Card = token.into();

        Self::upload_card(
            db,
            &card,
            player,
            OnBattlefield(NEXT_BATTLEFIELD_SEQ.fetch_add(1, Ordering::Relaxed)),
            true,
        )
    }

    #[must_use]
    pub(crate) fn token_copy_of(self, db: &mut Database, player: Owner) -> CardId {
        Self::upload_card(
            db,
            &self.original(db),
            player,
            OnBattlefield(NEXT_BATTLEFIELD_SEQ.fetch_add(1, Ordering::Relaxed)),
            true,
        )
    }

    pub(crate) fn original(self, db: &Database) -> Card {
        db.get::<Card>(self.0).cloned().unwrap()
    }

    fn upload_card<Location: Component + std::marker::Copy + std::fmt::Debug>(
        db: &mut Database,
        card: &Card,
        player: Owner,
        destination: Location,
        is_token: bool,
    ) -> CardId {
        let cardid = CardId(db.spawn_empty().id());
        Self::insert_components(db, cardid, card, player, destination, is_token);
        cardid
    }

    #[instrument(skip(db, card))]
    fn insert_components<Location: Component + std::marker::Copy + std::fmt::Debug>(
        db: &mut Database,
        cardid: CardId,
        card: &Card,
        player: Owner,
        destination: Location,
        is_token: bool,
    ) {
        let mut entity = db.entity_mut(cardid.0);

        entity.insert((
            card.clone(),
            destination,
            Name(card.name.clone()),
            OracleText(card.oracle_text.clone()),
            player,
            Controller::from(player),
            card.cost.clone(),
            Types(card.types.clone()),
            Subtypes(card.subtypes.clone()),
            Colors(card.colors.clone()),
            Keywords(card.keywords.clone()),
            Restrictions(card.restrictions.clone()),
            UniqueId::new(),
        ));

        if is_token {
            entity.insert(IsToken);
        }

        if let Some(reducer) = card.reducer.as_ref() {
            entity.insert(reducer.clone());
        }

        if let Some(ward) = card.ward.as_ref() {
            entity.insert(ward.clone());
        }

        if card.target_individually {
            entity.insert(TargetIndividually);
        }

        if card.etb_tapped {
            entity.insert(EtbTapped);
        }

        if let Some(power) = card.power {
            entity.insert(BasePower(power as i32));
        }

        if let Some(toughness) = card.toughness {
            entity.insert(BaseToughness(toughness as i32));
        }

        if card.cannot_be_countered {
            entity.insert(CannotBeCountered);
        }

        if card.keywords.contains_key(&Keyword::SplitSecond) {
            entity.insert(SplitSecond);
        }

        if !card.effects.is_empty() {
            entity.insert(Effects(card.effects.clone()));
        }

        if !card.modes.is_empty() {
            entity.insert(Modes(card.modes.clone()));
        }

        if !card.etb_abilities.is_empty() {
            let id = AbilityId::upload_ability(
                db,
                cardid,
                Ability::Etb {
                    effects: card.etb_abilities.clone(),
                },
            );

            db.entity_mut(cardid.0).insert(ETBAbilities(vec![id]));
        }

        if !card.mana_abilities.is_empty() {
            let mut ability_ids = vec![];
            for gain_mana in card.mana_abilities.iter() {
                let id = AbilityId::upload_ability(db, cardid, Ability::Mana(gain_mana.clone()));

                ability_ids.push(id);
            }

            if let Some(mut abilities) = db.get_mut::<ActivatedAbilities>(cardid.0) {
                abilities.extend(ability_ids);
            } else {
                db.entity_mut(cardid.0)
                    .insert(ActivatedAbilities(ability_ids));
            }
        }

        if !card.activated_abilities.is_empty() {
            let mut ability_ids = vec![];
            for ability in card.activated_abilities.iter() {
                let id = AbilityId::upload_ability(db, cardid, Ability::Activated(ability.clone()));

                ability_ids.push(id);
            }

            if let Some(mut abilities) = db.get_mut::<ActivatedAbilities>(cardid.0) {
                abilities.extend(ability_ids);
            } else {
                db.entity_mut(cardid.0)
                    .insert(ActivatedAbilities(ability_ids));
            }
        }

        if !card.static_abilities.is_empty() {
            db.entity_mut(cardid.0)
                .insert(StaticAbilities(card.static_abilities.clone()));
        }

        if let Some(enchant) = &card.enchant {
            let mut modifierids = vec![];
            for modifier in enchant.modifiers.iter() {
                let modifierid = ModifierId::upload_modifier(db, cardid, modifier, false);
                modifierids.push(modifierid);
            }

            let auraid = AuraId::from(db.auras.spawn((Modifiers(modifierids),)).id());

            db.entity_mut(cardid.0).insert(auraid);
        }

        if !card.triggered_abilities.is_empty() {
            let mut trigger_ids = vec![];
            for ability in card.triggered_abilities.iter() {
                trigger_ids.push(TriggerId::upload(db, ability, cardid, false));
            }

            db.entity_mut(cardid.0).insert(Triggers(trigger_ids));
        }

        if !card.replacement_effects.is_empty() {
            let mut ids = vec![];
            for effect in card.replacement_effects.iter() {
                ids.push(ReplacementEffectId::upload_replacement_effect(
                    db, effect, cardid,
                ));
            }

            db.entity_mut(cardid.0).insert(ReplacementEffects(ids));
        }

        if let Some(back) = &card.back_face {
            let id = CardId::upload_card(db, back, player, destination, is_token);
            id.move_to_limbo(db);

            db.entity_mut(id.0).insert(FrontFace(cardid));
            db.entity_mut(id.0).insert(BackFace(id));
            db.entity_mut(cardid.0).insert(FrontFace(cardid));
            db.entity_mut(cardid.0).insert(BackFace(id));
        }

        cardid.apply_modifiers_layered(db);
    }

    pub fn cost(self, db: &Database) -> &CastingCost {
        db.get::<CastingCost>(self.0).unwrap()
    }

    #[cfg(test)]
    pub(crate) fn valid_targets(
        self,
        db: &mut Database,
        already_chosen: &HashSet<ActiveTarget>,
    ) -> Vec<Vec<ActiveTarget>> {
        let mut targets = vec![];

        if let Some(aura_targets) = self.targets_for_aura(db) {
            targets.push(aura_targets);
        }

        let controller = self.controller(db);
        for effect in self.effects(db) {
            let effect = effect.into_effect(db, controller);
            targets.push(effect.valid_targets(db, self, controller, already_chosen));
        }

        for ability in self.activated_abilities(db) {
            targets.extend(self.targets_for_ability(db, ability, already_chosen));
        }

        targets
    }

    pub(crate) fn targets_for_ability(
        self,
        db: &mut Database,
        ability: AbilityId,
        already_chosen: &HashSet<ActiveTarget>,
    ) -> Vec<Vec<ActiveTarget>> {
        let mut targets = vec![];
        let ability = ability.ability(db);
        let controller = self.controller(db);
        if !ability.apply_to_self() {
            for effect in ability.into_effects() {
                let effect = effect.into_effect(db, controller);
                targets.push(effect.valid_targets(db, self, controller, already_chosen));
            }
        } else {
            targets.push(vec![ActiveTarget::Battlefield { id: self }])
        }

        targets
    }

    pub(crate) fn targets_for_aura(self, db: &mut Database) -> Option<Vec<ActiveTarget>> {
        if self.aura(db).is_some() {
            let mut targets = vec![];
            let controller = self.controller(db);
            for card in in_play::cards::<OnBattlefield>(db) {
                let card_restrictions = self.restrictions(db);
                if !card.passes_restrictions(
                    db,
                    self,
                    ControllerRestriction::Any,
                    &card_restrictions,
                ) {
                    continue;
                }

                if !card.can_be_targeted(db, controller) {
                    continue;
                }

                targets.push(ActiveTarget::Battlefield { id: card });
            }
            Some(targets)
        } else {
            None
        }
    }

    pub(crate) fn can_be_countered(
        self,
        db: &mut Database,
        caster: Controller,
        target: &SpellTarget,
    ) -> bool {
        if db.get::<CannotBeCountered>(self.0).is_some() {
            return false;
        }

        let SpellTarget {
            controller,
            types,
            subtypes,
        } = target;

        match controller {
            ControllerRestriction::You => {
                if caster != self.controller(db) {
                    return false;
                }
            }
            ControllerRestriction::Opponent => {
                if caster == self.controller(db) {
                    return false;
                }
            }
            ControllerRestriction::Any => {}
        };

        if !types.is_empty() && !self.types_intersect(db, types) {
            return false;
        }

        if !self.subtypes_intersect(db, subtypes) {
            return false;
        }

        let colors = self.colors(db);
        for (ability, ability_controller) in Battlefield::static_abilities(db) {
            match &ability {
                StaticAbility::GreenCannotBeCountered { controller } => {
                    if colors.contains(&Color::Green) {
                        match controller {
                            ControllerRestriction::You => {
                                if ability_controller == self.controller(db) {
                                    return false;
                                }
                            }
                            ControllerRestriction::Opponent => {
                                if ability_controller != self.controller(db) {
                                    return false;
                                }
                            }
                            ControllerRestriction::Any => {
                                return false;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        true
    }

    pub(crate) fn can_be_targeted(self, db: &Database, caster: Controller) -> bool {
        if self.shroud(db) {
            return false;
        }

        if self.hexproof(db) && self.controller(db) != caster {
            return false;
        }

        // TODO protection

        true
    }

    pub(crate) fn can_be_sacrificed(self, _db: &Database) -> bool {
        // TODO
        true
    }

    pub(crate) fn tapped(self, db: &Database) -> bool {
        db.get::<Tapped>(self.0).is_some()
    }

    pub(crate) fn tap(self, db: &mut Database) -> PendingResults {
        let mut pending = PendingResults::default();
        for trigger in TriggerId::active_triggers_of_source::<trigger_source::Tapped>(db) {
            let restrictions = trigger.restrictions(db);
            if self.passes_restrictions(
                db,
                trigger.listener(db),
                trigger.controller_restriction(db),
                &restrictions,
            ) {
                let listener = trigger.listener(db);
                pending.extend(Stack::move_trigger_to_stack(db, trigger, listener));
            }
        }

        db.entity_mut(self.0).insert(Tapped);

        pending
    }

    pub fn untap(self, db: &mut Database) {
        db.entity_mut(self.0).remove::<Tapped>();
    }

    pub(crate) fn clone_card<Location: Component + std::marker::Copy + std::fmt::Debug>(
        self,
        db: &mut Database,
        source: CardId,
        location: Location,
    ) {
        db.entity_mut(self.0).insert(Cloning(source.0));
        let controller = source.controller(db);
        Self::insert_components(
            db,
            self,
            &source.original(db).clone(),
            controller.into(),
            location,
            false,
        );
    }

    pub(crate) fn cloning(self, db: &Database) -> Option<Cloning> {
        db.get::<Cloning>(self.0).copied()
    }

    pub(crate) fn is_land(self, db: &Database) -> bool {
        self.types_intersect(db, &IndexSet::from([Type::Land, Type::BasicLand]))
    }

    pub(crate) fn manifest(self, db: &mut Database) {
        db.entity_mut(self.0).insert(Manifested).insert(FaceDown);
    }

    pub(crate) fn manifested(self, db: &Database) -> bool {
        db.get::<Manifested>(self.0).is_some()
    }

    pub(crate) fn is_permanent(self, db: &Database) -> bool {
        !self.types_intersect(db, &IndexSet::from([Type::Instant, Type::Sorcery]))
    }

    pub(crate) fn keywords(self, db: &Database) -> ::counter::Counter<Keyword> {
        db.get::<ModifiedKeywords>(self.0)
            .map(|t| t.0.clone())
            .or_else(|| db.get::<Keywords>(self.0).map(|t| t.0.clone()))
            .unwrap_or_default()
    }

    pub(crate) fn shroud(self, db: &Database) -> bool {
        self.keywords(db).contains_key(&Keyword::Shroud)
    }

    pub(crate) fn hexproof(self, db: &Database) -> bool {
        self.keywords(db).contains_key(&Keyword::Hexproof)
    }

    #[allow(unused)]
    pub(crate) fn flying(self, db: &Database) -> bool {
        self.keywords(db).contains_key(&Keyword::Flying)
    }

    pub(crate) fn vigilance(self, db: &Database) -> bool {
        self.keywords(db).contains_key(&Keyword::Vigilance)
    }

    pub fn name(self, db: &Database) -> String {
        db.get::<Name>(self.0).unwrap().0.clone()
    }

    pub fn oracle_text(self, db: &Database) -> String {
        db.get::<OracleText>(self.0).unwrap().0.clone()
    }

    pub(crate) fn has_flash(&self, db: &Database) -> bool {
        self.keywords(db).contains_key(&Keyword::Flash)
    }

    pub(crate) fn abilities_text(self, db: &mut Database) -> String {
        self.activated_abilities(db)
            .into_iter()
            .map(|ability| ability.text(db))
            .join("\n")
    }

    pub fn pt_text(&self, db: &Database) -> Option<String> {
        let power = self.power(db);
        let toughness = self.toughness(db);

        if let Some(power) = power {
            let toughness = toughness.expect("Should never have toughness without power");
            Some(format!("{}/{}", power, toughness))
        } else {
            None
        }
    }

    pub fn modified_by(&self, db: &mut Database) -> Vec<String> {
        let mut results = vec![];

        let modifiers = self.modifiers(db);
        for modifier in modifiers {
            results.push(modifier.source(db).name(db));
        }

        results
    }

    pub fn triggers_text(self, db: &mut Database) -> Vec<String> {
        let triggers = self.triggers(db);

        let mut results = vec![];
        for trigger in triggers {
            results.push(trigger.text(db))
        }

        results
    }

    pub(crate) fn reveal(self, db: &mut Database) {
        db.entity_mut(self.0).insert(Revealed);
    }

    pub(crate) fn settle(self, db: &mut Database) {
        db.entity_mut(self.0).insert(Settled);
    }

    pub(crate) fn cast_location<Location: Component + Default>(self, db: &mut Database) {
        db.entity_mut(self.0).insert(Location::default());
    }

    pub(crate) fn cascade(self, db: &Database) -> usize {
        self.keywords(db)
            .get(&Keyword::Cascade)
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn exiled_with_cascade(db: &mut Database) -> Vec<CardId> {
        db.query_filtered::<Entity, (With<InExile>, With<exile_reason::Cascade>)>()
            .iter(db)
            .map(Self)
            .collect_vec()
    }

    pub(crate) fn target_individually(self, db: &Database) -> bool {
        db.get::<TargetIndividually>(self.0).is_some()
    }

    pub(crate) fn set_x(self, db: &mut Database, x_is: usize) {
        db.entity_mut(self.0).insert(PaidX(x_is));
    }

    pub(crate) fn get_x(self, db: &Database) -> usize {
        db.get::<PaidX>(self.0)
            .map(|paid| paid.0)
            .unwrap_or_default()
    }

    pub(crate) fn mana_from_source(self, db: &mut Database, sources: &[ManaSource]) {
        let mut sourced = HashMap::default();
        for source in sources.iter().copied() {
            *sourced.entry(source).or_default() += 1
        }

        db.entity_mut(self.0).insert(SourcedMana(sourced));
    }

    pub(crate) fn get_mana_from_sources(self, db: &Database) -> Option<SourcedMana> {
        db.get::<SourcedMana>(self.0).cloned()
    }

    pub(crate) fn attacking(self, db: &Database) -> bool {
        db.get::<Attacking>(self.0).is_some()
    }

    pub(crate) fn set_attacking(self, db: &mut Database, target: Owner) {
        db.entity_mut(self.0).insert(Attacking(target));
    }

    pub(crate) fn all_attackers(db: &mut Database) -> Vec<(CardId, Owner)> {
        db.query::<(Entity, &Attacking)>()
            .iter(db)
            .map(|(e, attacking)| (Self(e), attacking.0))
            .collect_vec()
    }

    pub(crate) fn clear_all_attacking(db: &mut Database) {
        for card in db
            .query_filtered::<Entity, With<Attacking>>()
            .iter(db)
            .collect_vec()
        {
            db.entity_mut(card).remove::<Attacking>();
        }
    }

    pub(crate) fn can_attack(self, db: &Database) -> bool {
        self.types_intersect(db, &IndexSet::from([Type::Creature]))
            && !self
                .static_abilities(db)
                .into_iter()
                .any(|ability| matches!(ability, StaticAbility::PreventAttacks))
    }

    pub(crate) fn exile_source(self, db: &Database) -> ExiledWith {
        db.get::<ExiledWith>(self.0).copied().unwrap()
    }

    pub(crate) fn until_source_leaves_battlefield(self, db: &Database) -> bool {
        db.get::<UntilSourceLeavesBattlefield>(self.0).is_some()
    }

    pub(crate) fn until_end_of_turn(self, db: &Database) -> bool {
        db.get::<UntilEndOfTurn>(self.0).is_some()
    }

    pub(crate) fn ward(self, db: &mut Database) -> Option<&Ward> {
        db.get::<Ward>(self.0)
    }

    pub(crate) fn cost_reducer(&self, db: &Database) -> Option<CostReducer> {
        db.get::<CostReducer>(self.0).cloned()
    }

    pub(crate) fn battle_cry(&self, db: &Database) -> bool {
        self.keywords(db).contains_key(&Keyword::BattleCry)
    }
}

pub(crate) fn target_from_location(db: &mut Database, card: CardId) -> ActiveTarget {
    if card.is_in_location::<OnBattlefield>(db) {
        ActiveTarget::Battlefield { id: card }
    } else if card.is_in_location::<InGraveyard>(db) {
        ActiveTarget::Graveyard { id: card }
    } else if card.is_in_location::<InLibrary>(db) {
        ActiveTarget::Library { id: card }
    } else {
        todo!()
    }
}
