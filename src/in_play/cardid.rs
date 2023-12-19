use std::{collections::HashSet, sync::atomic::Ordering};

use bevy_ecs::{component::Component, entity::Entity, query::With};
use derive_more::From;
use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    abilities::{
        Ability, ActivatedAbilities, ETBAbilities, GainMana, ModifiedActivatedAbilities,
        ModifiedETBAbilities, ModifiedStaticAbilities, ModifiedTriggers, StaticAbilities,
        StaticAbility, Triggers,
    },
    battlefield::{compute_deck_targets, compute_graveyard_targets, Battlefield},
    card::{
        keyword::SplitSecond, ActivatedAbilityModifier, AddPower, AddToughness, BasePower,
        BaseToughness, CannotBeCountered, Card, Color, Colors, EtbAbilityModifier, EtbTapped,
        Keyword, Keywords, MarkedDamage, ModifiedBasePower, ModifiedBaseToughness, ModifiedColors,
        ModifiedKeywords, ModifyKeywords, Name, OracleText, Revealed, StaticAbilityModifier,
        TriggeredAbilityModifier,
    },
    controller::ControllerRestriction,
    cost::CastingCost,
    effects::{
        effect_duration::UntilSourceLeavesBattlefield, AnyEffect, BattlefieldModifier, DealDamage,
        DynamicPowerToughness, Effect, Effects, Mill, ReplacementEffects,
        ReturnFromGraveyardToBattlefield, ReturnFromGraveyardToLibrary, Token, TutorLibrary,
    },
    in_play::{
        self, cast_from, exile_reason, AbilityId, Active, AuraId, CastFrom, CounterId, Database,
        EntireBattlefield, ExileReason, FaceDown, Global, InExile, InGraveyard, InHand, InLibrary,
        InStack, IsToken, Manifested, ModifierId, ModifierSeq, Modifiers, Modifying, OnBattlefield,
        ReplacementEffectId, Tapped, TriggerId, UniqueId, NEXT_BATTLEFIELD_SEQ, NEXT_GRAVEYARD_SEQ,
        NEXT_HAND_SEQ, NEXT_STACK_SEQ,
    },
    player::{AllPlayers, Controller, Owner},
    stack::{ActiveTarget, Settled, Stack, Targets},
    targets::{Comparison, Restriction, Restrictions, SpellTarget},
    types::{ModifiedSubtypes, ModifiedTypes, Subtype, Subtypes, Type, Types},
    Cards,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Component)]
pub struct CardId(pub(super) Entity);

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Component)]
pub struct Cloning(pub(super) Entity);

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

    pub fn is_in_location<Location: Component + Ord>(self, database: &Database) -> bool {
        database.get::<Location>(self.0).is_some()
    }

    pub fn is_token(self, db: &mut Database) -> bool {
        db.get::<IsToken>(self.0).is_some()
    }

    pub fn facedown(self, db: &mut Database) -> bool {
        db.get::<FaceDown>(self.0).is_some()
    }

    pub fn move_to_hand(self, db: &mut Database) {
        if self.is_token(db) {
            db.cards.despawn(self.0);
        } else {
            self.remove_all_modifiers(db);
            TriggerId::deactivate_all_for_card(db, self);
            ReplacementEffectId::deactivate_all_for_card(db, self);
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
                .insert(InHand(NEXT_HAND_SEQ.fetch_add(1, Ordering::Relaxed)));
        }
    }

    pub fn move_to_stack(
        self,
        db: &mut Database,
        targets: Vec<Vec<ActiveTarget>>,
        from: Option<CastFrom>,
    ) {
        if Stack::split_second(db) {
            return;
        }

        if self.is_token(db) {
            db.cards.despawn(self.0);
        } else {
            self.remove_all_modifiers(db);
            TriggerId::deactivate_all_for_card(db, self);
            ReplacementEffectId::deactivate_all_for_card(db, self);
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
                .insert(InStack(NEXT_STACK_SEQ.fetch_add(1, Ordering::Relaxed)))
                .insert(Targets(targets));

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

    pub fn cast_from_hand(self, db: &Database) -> bool {
        db.get::<InStack>(self.0).is_some() && db.get::<cast_from::Hand>(self.0).is_some()
    }

    pub fn move_to_battlefield(self, db: &mut Database) {
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
            .insert(OnBattlefield(
                NEXT_BATTLEFIELD_SEQ.fetch_add(1, Ordering::Relaxed),
            ));

        TriggerId::activate_all_for_card(db, self);
        ReplacementEffectId::activate_all_for_card(db, self);
    }

    pub fn move_to_graveyard(self, db: &mut Database) {
        if self.is_token(db) {
            db.cards.despawn(self.0);
        } else {
            self.remove_all_modifiers(db);
            TriggerId::deactivate_all_for_card(db, self);
            ReplacementEffectId::deactivate_all_for_card(db, self);
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
                .insert(InGraveyard(
                    NEXT_GRAVEYARD_SEQ.fetch_add(1, Ordering::Relaxed),
                ));
        }
    }

    pub fn move_to_library(self, db: &mut Database) -> bool {
        if self.is_token(db) {
            db.cards.despawn(self.0);
            false
        } else {
            self.remove_all_modifiers(db);
            TriggerId::deactivate_all_for_card(db, self);
            ReplacementEffectId::deactivate_all_for_card(db, self);
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
                .insert(InLibrary);
            true
        }
    }

    pub fn move_to_exile(self, db: &mut Database, reason: Option<ExileReason>) {
        if self.is_token(db) {
            db.cards.despawn(self.0);
        } else {
            self.remove_all_modifiers(db);
            TriggerId::deactivate_all_for_card(db, self);
            ReplacementEffectId::deactivate_all_for_card(db, self);
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
                .insert(InExile);

            if let Some(reason) = reason {
                match reason {
                    ExileReason::Cascade => {
                        entity.insert(exile_reason::Cascade);
                    }
                }
            }
        }
    }

    pub fn remove_all_modifiers(self, db: &mut Database) {
        for mut modifying in db
            .modifiers
            .query::<&mut Modifying>()
            .iter_mut(&mut db.modifiers)
        {
            modifying.remove(&self);
        }
    }

    pub fn modifiers(self, db: &mut Database) -> Vec<ModifierId> {
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

    pub fn deactivate_modifiers(self, db: &mut Database) {
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

    pub fn apply_modifiers_layered(self, db: &mut Database) {
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

        let facedown = self.facedown(db);

        let mut base_power = if facedown {
            Some(2)
        } else {
            db.get::<BasePower>(self.0).map(|bp| bp.0)
        };
        let mut base_toughness = if facedown {
            Some(2)
        } else {
            db.get::<BaseToughness>(self.0).map(|bt| bt.0)
        };
        let mut types = if facedown {
            IndexSet::from([Type::Creature])
        } else {
            db.get::<Types>(self.0).unwrap().0.clone()
        };
        let mut subtypes = if facedown {
            Default::default()
        } else {
            db.get::<Subtypes>(self.0).unwrap().0.clone()
        };
        let mut keywords = if facedown {
            ::counter::Counter::default()
        } else {
            db.get::<Keywords>(self.0).unwrap().0.clone()
        };
        let mut colors: HashSet<Color> = if facedown {
            HashSet::default()
        } else {
            db.get::<Colors>(self.0)
                .unwrap()
                .0
                .union(&db.get::<CastingCost>(self.0).unwrap().colors())
                .copied()
                .collect()
        };
        let mut triggers = if facedown {
            vec![]
        } else {
            db.get::<Triggers>(self.0)
                .cloned()
                .map(|t| t.0)
                .unwrap_or_default()
        };
        let mut etb_abilities = if facedown {
            vec![]
        } else {
            db.get::<ETBAbilities>(self.0)
                .cloned()
                .map(|t| t.0)
                .unwrap_or_default()
        };
        let mut static_abilities = if facedown {
            vec![]
        } else {
            db.get::<StaticAbilities>(self.0)
                .cloned()
                .map(|s| s.0)
                .unwrap_or_default()
        };
        let mut activated_abilities = if facedown {
            vec![]
        } else {
            db.get::<ActivatedAbilities>(self.0)
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
                if !self.passes_restrictions_given_types(
                    db,
                    source,
                    controller_restriction,
                    &restrictions,
                    &types,
                    &subtypes,
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
                if !self.passes_restrictions_given_types(
                    db,
                    source,
                    controller_restriction,
                    &restrictions,
                    &types,
                    &subtypes,
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
                if !self.passes_restrictions_given_types(
                    db,
                    source,
                    controller_restriction,
                    &restrictions,
                    &types,
                    &subtypes,
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
                    StaticAbilityModifier::Add(add) => static_abilities.push(add.clone()),
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
                if !self.passes_restrictions_given_types(
                    db,
                    source,
                    controller_restriction,
                    &restrictions,
                    &types,
                    &subtypes,
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

        if let Some(bp) = base_power {
            db.entity_mut(self.0).insert(ModifiedBasePower(bp));
        }
        if let Some(bt) = base_toughness {
            db.entity_mut(self.0).insert(ModifiedBaseToughness(bt));
        }

        for trigger in triggers.iter() {
            trigger.add_listener(db, self);
        }

        db.entity_mut(self.0)
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

    pub fn etb_abilities(self, db: &mut Database) -> Vec<AbilityId> {
        db.get::<ModifiedETBAbilities>(self.0)
            .cloned()
            .map(|m| m.0)
            .or_else(|| db.get::<ETBAbilities>(self.0).cloned().map(|m| m.0))
            .unwrap_or_default()
    }

    pub fn static_abilities(self, db: &mut Database) -> Vec<StaticAbility> {
        db.get::<ModifiedStaticAbilities>(self.0)
            .cloned()
            .map(|m| m.0)
            .or_else(|| db.get::<StaticAbilities>(self.0).cloned().map(|m| m.0))
            .unwrap_or_default()
    }
    pub fn activated_abilities(self, db: &mut Database) -> Vec<AbilityId> {
        db.get::<ModifiedActivatedAbilities>(self.0)
            .cloned()
            .map(|m| m.0)
            .or_else(|| db.get::<ActivatedAbilities>(self.0).cloned().map(|m| m.0))
            .unwrap_or_default()
    }

    pub fn controller(self, db: &mut Database) -> Controller {
        db.get::<Controller>(self.0).copied().unwrap()
    }

    pub fn owner(self, db: &mut Database) -> Owner {
        db.get::<Owner>(self.0).copied().unwrap()
    }

    pub fn etb_tapped(self, db: &Database) -> bool {
        db.get::<EtbTapped>(self.0).is_some()
    }

    pub fn apply_modifier(self, db: &mut Database, modifier: ModifierId) {
        db.modifiers
            .get_mut::<Modifying>(modifier.into())
            .unwrap()
            .insert(self);
        modifier.activate(db);
        self.apply_modifiers_layered(db);
    }

    pub fn effects_count(self, db: &mut Database) -> usize {
        let aura_count = self.aura(db).map(|_| 1).unwrap_or_default();
        aura_count
            + db.get::<Effects>(self.0)
                .map(|effects| effects.len())
                .unwrap_or_default()
    }

    pub fn effects(self, db: &mut Database) -> Vec<AnyEffect> {
        db.get::<Effects>(self.0).cloned().unwrap_or_default().0
    }

    pub fn needs_targets(self, db: &mut Database) -> Vec<usize> {
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

    pub fn wants_targets(&self, db: &mut Database) -> Vec<usize> {
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

    pub fn passes_restrictions(
        self,
        db: &mut Database,
        source: CardId,
        controller_restriction: ControllerRestriction,
        restrictions: &[Restriction],
    ) -> bool {
        let types = self.types(db);
        let subtypes = self.subtypes(db);

        self.passes_restrictions_given_types(
            db,
            source,
            controller_restriction,
            restrictions,
            &types,
            &subtypes,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn passes_restrictions_given_types(
        self,
        db: &mut Database,
        source: CardId,
        controller_restriction: ControllerRestriction,
        restrictions: &[Restriction],
        self_types: &IndexSet<Type>,
        self_subtypes: &IndexSet<Subtype>,
    ) -> bool {
        match controller_restriction {
            ControllerRestriction::Any => {}
            ControllerRestriction::You => {
                let source_controller = source.controller(db);
                if source_controller != self.controller(db) {
                    return false;
                }
            }
            ControllerRestriction::Opponent => {
                let source_controller = source.controller(db);
                if source_controller == self.controller(db) {
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
                Restriction::Toughness(comparison) => {
                    let toughness = self.toughness(db);
                    if toughness.is_none() {
                        return false;
                    }

                    let toughness = toughness.unwrap();
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
                    let controller = self.controller(db);
                    let colors = Battlefield::controlled_colors(db, controller);
                    if !(colors.contains(&Color::Green) || colors.contains(&Color::Black)) {
                        return false;
                    }
                }
                Restriction::ControllerHandEmpty => {
                    if self.controller(db).has_cards::<InHand>(db) {
                        return false;
                    }
                }
                Restriction::OfColor(colors) => {
                    let self_colors = self.colors(db);
                    if self_colors.is_disjoint(colors) {
                        return false;
                    }
                }
                Restriction::Cmc(comparison) => {
                    let cmc = self.cost(db).cmc() as i32;
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
                Restriction::CastFromHand => {}
            }
        }

        true
    }

    pub fn apply_aura(self, db: &mut Database, aura: AuraId) {
        let modifiers = aura.modifiers(db);

        for modifier in modifiers.iter() {
            self.apply_modifier(db, *modifier);
        }
    }

    pub fn marked_damage(self, db: &mut Database) -> i32 {
        db.get::<MarkedDamage>(self.0)
            .copied()
            .unwrap_or_default()
            .0
    }

    pub fn mark_damage(self, db: &mut Database, amount: usize) {
        if let Some(mut marked) = db.get_mut::<MarkedDamage>(self.0) {
            **marked += amount as i32;
        } else {
            db.entity_mut(self.0).insert(MarkedDamage(amount as i32));
        }
    }

    pub fn clear_damage(self, db: &mut Database) {
        if let Some(mut marked) = db.get_mut::<MarkedDamage>(self.0) {
            **marked = 0;
        }
    }

    pub fn power(self, db: &Database) -> Option<i32> {
        db.get::<ModifiedBasePower>(self.0)
            .map(|bp| bp.0)
            .or_else(|| db.get::<BasePower>(self.0).map(|bp| bp.0))
            .map(|bp| bp + db.get::<AddPower>(self.0).map(|a| a.0).unwrap_or_default())
    }

    pub fn toughness(self, db: &Database) -> Option<i32> {
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

    pub fn aura(self, db: &mut Database) -> Option<AuraId> {
        db.get::<AuraId>(self.0).copied()
    }

    pub fn colors(self, db: &mut Database) -> HashSet<Color> {
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

    pub fn types_intersect(self, db: &mut Database, types: &IndexSet<Type>) -> bool {
        types.is_empty() || !self.types(db).is_disjoint(types)
    }

    pub fn types(self, db: &mut Database) -> IndexSet<Type> {
        db.get::<ModifiedTypes>(self.0)
            .map(|t| t.0.clone())
            .or_else(|| db.get::<Types>(self.0).map(|t| t.0.clone()))
            .unwrap_or_default()
    }

    pub fn subtypes_intersect(self, db: &mut Database, types: &IndexSet<Subtype>) -> bool {
        types.is_empty() || !self.subtypes(db).is_disjoint(types)
    }

    pub fn subtypes(self, db: &mut Database) -> IndexSet<Subtype> {
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
    pub fn upload_token(db: &mut Database, player: Owner, token: Token) -> CardId {
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
    pub fn token_copy_of(self, db: &mut Database, player: Owner) -> CardId {
        Self::upload_card(
            db,
            &self.original(db),
            player,
            OnBattlefield(NEXT_BATTLEFIELD_SEQ.fetch_add(1, Ordering::Relaxed)),
            true,
        )
    }

    pub fn original(self, db: &Database) -> Card {
        db.get::<Card>(self.0).cloned().unwrap()
    }

    fn upload_card<Location: Component>(
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

    fn insert_components<Location: Component>(
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
            UniqueId::new(),
        ));

        if is_token {
            entity.insert(IsToken);
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

        if !card.etb_abilities.is_empty() {
            let id = AbilityId::upload_ability(
                db,
                cardid,
                Ability::ETB {
                    effects: card.etb_abilities.clone(),
                },
            );

            db.entity_mut(cardid.0).insert(ETBAbilities(vec![id]));
        }

        if !card.activated_abilities.is_empty() {
            let mut ability_ids = vec![];
            for ability in card.activated_abilities.iter() {
                let id = AbilityId::upload_ability(db, cardid, Ability::Activated(ability.clone()));

                ability_ids.push(id);
            }

            db.entity_mut(cardid.0)
                .insert(ActivatedAbilities(ability_ids));
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

            let auraid = AuraId::from(
                db.auras
                    .spawn((
                        Modifiers(modifierids),
                        Restrictions(enchant.restrictions.clone()),
                    ))
                    .id(),
            );

            db.entity_mut(cardid.0).insert(auraid);
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

        cardid.apply_modifiers_layered(db);
    }

    pub fn cost(self, db: &Database) -> &CastingCost {
        db.get::<CastingCost>(self.0).unwrap()
    }

    pub fn valid_targets(self, db: &mut Database) -> Vec<Vec<ActiveTarget>> {
        let mut targets = vec![];

        if let Some(aura_targets) = self.targets_for_aura(db) {
            targets.push(aura_targets);
        }

        let creatures = Battlefield::creatures(db);
        let controller = self.controller(db);
        for effect in self.effects(db) {
            let effect = effect.into_effect(db, controller);
            targets.push(self.targets_for_effect(db, controller, &effect, &creatures));
        }

        for ability in self.activated_abilities(db) {
            targets.extend(self.targets_for_ability(db, ability, &creatures));
        }

        targets
    }

    fn targets_for_damage(
        self,
        db: &mut Database,
        creatures: &[CardId],
        dmg: &DealDamage,
        targets: &mut Vec<ActiveTarget>,
    ) {
        for creature in creatures.iter() {
            let controller = self.controller(db);
            if creature.can_be_targeted(db, controller)
                && creature.passes_restrictions(
                    db,
                    self,
                    ControllerRestriction::Any,
                    &dmg.restrictions,
                )
            {
                targets.push(ActiveTarget::Battlefield { id: *creature });
            }
        }
        for player in AllPlayers::all_players_in_db(db) {
            // TODO player hexproof, non-all-target-damage
            targets.push(ActiveTarget::Player { id: player });
        }
    }

    pub fn targets_for_ability(
        self,
        db: &mut Database,
        ability: AbilityId,
        creatures: &[CardId],
    ) -> Vec<Vec<ActiveTarget>> {
        let mut targets = vec![];
        let ability = ability.ability(db);
        let controller = self.controller(db);
        if !ability.apply_to_self() {
            for effect in ability.into_effects() {
                let effect = effect.into_effect(db, controller);
                targets.push(self.targets_for_effect(db, controller, &effect, creatures));
            }
        } else {
            targets.push(vec![ActiveTarget::Battlefield { id: self }])
        }

        targets
    }

    pub fn targets_for_aura(self, db: &mut Database) -> Option<Vec<ActiveTarget>> {
        if let Some(aura) = self.aura(db) {
            let mut targets = vec![];
            let controller = self.controller(db);
            for card in in_play::cards::<OnBattlefield>(db) {
                if card.passes_restrictions(
                    db,
                    self,
                    ControllerRestriction::Any,
                    &aura.restrictions(db),
                ) && card.can_be_targeted(db, controller)
                {
                    targets.push(ActiveTarget::Battlefield { id: card });
                }
            }
            Some(targets)
        } else {
            None
        }
    }

    pub fn targets_for_effect(
        self,
        db: &mut Database,
        controller: Controller,
        effect: &Effect,
        creatures: &[CardId],
    ) -> Vec<ActiveTarget> {
        let mut targets = vec![];
        match effect {
            Effect::CounterSpell { target } => {
                targets_for_counterspell(db, controller, target, &mut targets);
            }
            Effect::BattlefieldModifier(_) => {}
            Effect::ControllerDrawCards(_) => {}
            Effect::Equip(_) => {
                targets_for_battlefield_modifier(
                    db,
                    self,
                    None,
                    creatures,
                    controller,
                    &mut targets,
                );
            }
            Effect::CreateToken(_) => {}
            Effect::DealDamage(dmg) => {
                self.targets_for_damage(db, creatures, dmg, &mut targets);
            }
            Effect::ExileTargetCreature => {
                for creature in creatures.iter() {
                    if creature.can_be_targeted(db, controller) {
                        targets.push(ActiveTarget::Battlefield { id: *creature });
                    }
                }
            }
            Effect::ExileTargetCreatureManifestTopOfLibrary => {
                for creature in creatures.iter() {
                    if creature.can_be_targeted(db, controller) {
                        targets.push(ActiveTarget::Battlefield { id: *creature });
                    }
                }
            }
            Effect::TargetToTopOfLibrary { restrictions } => {
                for target in in_play::cards::<OnBattlefield>(db) {
                    if target.passes_restrictions(
                        db,
                        self,
                        ControllerRestriction::Any,
                        restrictions,
                    ) {
                        targets.push(ActiveTarget::Battlefield { id: target });
                    }
                }
            }
            Effect::GainCounter(_) => {}
            Effect::ModifyCreature(modifier) => {
                targets_for_battlefield_modifier(
                    db,
                    self,
                    Some(modifier),
                    creatures,
                    controller,
                    &mut targets,
                );
            }
            Effect::ControllerLosesLife(_) => {}
            Effect::CopyOfAnyCreatureNonTargeting => {
                for creature in creatures.iter() {
                    targets.push(ActiveTarget::Battlefield { id: *creature });
                }
            }
            Effect::Mill(Mill { target, .. }) => {
                targets.extend(
                    match target {
                        ControllerRestriction::Any => AllPlayers::all_players_in_db(db),
                        ControllerRestriction::You => HashSet::from([controller.into()]),
                        ControllerRestriction::Opponent => {
                            let mut all = AllPlayers::all_players_in_db(db);
                            all.remove(&controller.into());
                            all
                        }
                    }
                    .into_iter()
                    .map(|player| ActiveTarget::Player { id: player }),
                );
            }
            Effect::ReturnFromGraveyardToBattlefield(ReturnFromGraveyardToBattlefield {
                types,
                ..
            }) => {
                targets.extend(
                    compute_graveyard_targets(db, ControllerRestriction::You, self, types)
                        .into_iter()
                        .map(|card| ActiveTarget::Graveyard { id: card }),
                );
            }
            Effect::ReturnFromGraveyardToLibrary(ReturnFromGraveyardToLibrary {
                controller,
                types,
                ..
            }) => {
                targets.extend(
                    compute_graveyard_targets(db, *controller, self, types)
                        .into_iter()
                        .map(|card| ActiveTarget::Graveyard { id: card }),
                );
            }
            Effect::TutorLibrary(TutorLibrary { restrictions, .. }) => {
                targets.extend(
                    compute_deck_targets(db, controller, restrictions)
                        .into_iter()
                        .map(|card| ActiveTarget::Library { id: card }),
                );
            }
            Effect::CreateTokenCopy { .. } => {
                for creature in creatures.iter() {
                    if creature.can_be_targeted(db, controller) {
                        targets.push(ActiveTarget::Battlefield { id: *creature })
                    }
                }
            }
            Effect::ReturnSelfToHand => {}
            Effect::RevealEachTopOfLibrary(_) => {}
            Effect::UntapThis => {}
            Effect::Cascade => {}
        }

        targets
    }

    pub fn can_be_countered(
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
                StaticAbility::BattlefieldModifier(_) => {}
                StaticAbility::ExtraLandsPerTurn(_) => {}
            }
        }

        true
    }

    pub fn can_be_targeted(self, db: &mut Database, caster: Controller) -> bool {
        if self.shroud(db) {
            return false;
        }

        if self.hexproof(db) && self.controller(db) != caster {
            return false;
        }

        // TODO protection

        true
    }

    pub fn can_be_sacrificed(self, _db: &mut Database) -> bool {
        // TODO
        true
    }

    pub fn tapped(self, db: &mut Database) -> bool {
        db.get::<Tapped>(self.0).is_some()
    }

    pub fn tap(self, db: &mut Database) {
        db.entity_mut(self.0).insert(Tapped);
    }

    pub fn untap(self, db: &mut Database) {
        db.entity_mut(self.0).remove::<Tapped>();
    }

    pub fn clone_card<Location: Component>(
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

    pub fn cloning(self, db: &mut Database) -> Option<Cloning> {
        db.get::<Cloning>(self.0).copied()
    }

    pub fn is_land(self, db: &mut Database) -> bool {
        self.types_intersect(db, &IndexSet::from([Type::Land, Type::BasicLand]))
    }

    pub fn manifest(self, db: &mut Database) {
        db.entity_mut(self.0).insert(Manifested).insert(FaceDown);
    }

    pub fn manifested(self, db: &Database) -> bool {
        db.get::<Manifested>(self.0).is_some()
    }

    pub fn is_permanent(self, db: &mut Database) -> bool {
        !self.types_intersect(db, &IndexSet::from([Type::Instant, Type::Sorcery]))
    }

    pub fn keywords(self, db: &Database) -> ::counter::Counter<Keyword> {
        db.get::<ModifiedKeywords>(self.0)
            .map(|t| t.0.clone())
            .or_else(|| db.get::<Keywords>(self.0).map(|t| t.0.clone()))
            .unwrap_or_default()
    }

    pub fn shroud(self, db: &mut Database) -> bool {
        self.keywords(db).contains_key(&Keyword::Shroud)
    }

    pub fn hexproof(self, db: &mut Database) -> bool {
        self.keywords(db).contains_key(&Keyword::Hexproof)
    }

    pub fn flying(self, db: &mut Database) -> bool {
        self.keywords(db).contains_key(&Keyword::Flying)
    }

    pub fn vigilance(self, db: &mut Database) -> bool {
        self.keywords(db).contains_key(&Keyword::Vigilance)
    }

    pub fn name(self, db: &Database) -> String {
        db.get::<Name>(self.0).unwrap().0.clone()
    }

    pub fn oracle_text(self, db: &Database) -> String {
        db.get::<OracleText>(self.0).unwrap().0.clone()
    }

    pub fn name_ref(self, db: &Database) -> &String {
        &db.get::<Name>(self.0).unwrap().0
    }

    pub fn oracle_text_ref(self, db: &Database) -> &String {
        &db.get::<OracleText>(self.0).unwrap().0
    }

    pub fn split_second(self, db: &mut Database) -> bool {
        db.get::<SplitSecond>(self.0).is_some()
    }

    pub fn has_flash(&self, db: &Database) -> bool {
        self.keywords(db).contains_key(&Keyword::Flash)
    }

    pub fn cannot_be_countered(&self, db: &mut Database) -> bool {
        db.get::<CannotBeCountered>(self.0).is_some()
    }

    pub fn abilities_text(self, db: &mut Database) -> String {
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
        let triggers = TriggerId::all_for_card(db, self);

        let mut results = vec![];
        for trigger in triggers {
            results.push(trigger.text(db))
        }

        results
    }

    pub fn reveal(self, db: &mut Database) {
        db.entity_mut(self.0).insert(Revealed);
    }

    pub fn settle(self, db: &mut Database) {
        db.entity_mut(self.0).insert(Settled);
    }

    pub fn cast_location<Location: Component + Default>(self, db: &mut Database) {
        db.entity_mut(self.0).insert(Location::default());
    }

    pub fn cascade(self, db: &Database) -> usize {
        self.keywords(db)
            .get(&Keyword::Cascade)
            .copied()
            .unwrap_or_default()
    }

    pub fn exiled_with_cascade(db: &mut Database) -> Vec<CardId> {
        db.query_filtered::<Entity, (With<InExile>, With<exile_reason::Cascade>)>()
            .iter(db)
            .map(Self)
            .collect_vec()
    }
}

fn targets_for_counterspell(
    db: &mut Database,
    caster: Controller,
    target: &SpellTarget,
    targets: &mut Vec<ActiveTarget>,
) {
    let cards_in_stack = db
        .query::<(Entity, &InStack)>()
        .iter(db)
        .map(|(entity, in_stack)| (CardId(entity), *in_stack))
        .sorted_by_key(|(_, in_stack)| *in_stack)
        .collect_vec();

    for (card, stack_id) in cards_in_stack.into_iter() {
        if card.can_be_countered(db, caster, target) {
            targets.push(ActiveTarget::Stack { id: stack_id });
        }
    }
}

fn targets_for_battlefield_modifier(
    db: &mut Database,
    source: CardId,
    modifier: Option<&BattlefieldModifier>,
    creatures: &[CardId],
    caster: Controller,
    targets: &mut Vec<ActiveTarget>,
) {
    for creature in creatures.iter() {
        if creature.can_be_targeted(db, caster)
            && (modifier.is_none()
                || creature.passes_restrictions(
                    db,
                    source,
                    modifier.unwrap().controller,
                    &modifier.unwrap().restrictions,
                ))
        {
            targets.push(ActiveTarget::Battlefield { id: *creature });
        }
    }
}
