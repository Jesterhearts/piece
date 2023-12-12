use std::{collections::HashSet, sync::atomic::Ordering};

use bevy_ecs::{component::Component, entity::Entity, query::With};
use derive_more::{Deref, From};
use itertools::Itertools;

use crate::{
    abilities::{
        Ability, ActivatedAbilities, ETBAbilities, ETBAbility, GainMana,
        ModifiedActivatedAbilities, ModifiedETBAbilities, ModifiedStaticAbilities,
        ModifiedTriggers, StaticAbilities, StaticAbility, TriggerListeners, Triggers,
    },
    battlefield::Battlefield,
    card::{
        ActivatedAbilityModifier, AddPower, AddToughness, BasePower, BaseToughness,
        CannotBeCountered, Card, Color, Colors, EtbAbilityModifier, Keyword, Keywords,
        MarkedDamage, ModifiedBasePower, ModifiedBaseToughness, ModifiedColors, ModifiedKeywords,
        ModifyKeywords, Name, SplitSecond, StaticAbilityModifier, TriggeredAbilityModifier,
    },
    controller::ControllerRestriction,
    cost::CastingCost,
    effects::{
        AnyEffect, DealDamage, DynamicPowerToughness, Effect, Effects, ReplacementEffects, Token,
        UntilSourceLeavesBattlefield,
    },
    in_play::{
        targets_for_battlefield_modifier, targets_for_counterspell, upload_modifier, AbilityId,
        Active, AuraId, CounterId, Database, EntireBattlefield, FaceDown, Global, InExile,
        InGraveyard, InHand, InLibrary, InStack, IsToken, Manifested, ModifierId, ModifierSeq,
        Modifiers, Modifying, OnBattlefield, ReplacementEffectId, Tapped, TriggerId,
        NEXT_BATTLEFIELD_SEQ, NEXT_GRAVEYARD_SEQ, NEXT_HAND_SEQ, NEXT_STACK_SEQ,
    },
    player::{Controller, Owner},
    stack::{ActiveTarget, Stack, Targets},
    targets::{Comparison, Restriction, Restrictions, SpellTarget},
    triggers::{source, TriggerSource},
    types::{ModifiedSubtypes, ModifiedTypes, Subtype, Subtypes, Type, Types},
    Cards,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Deref, Component)]
pub struct CardId(pub(super) Entity);

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Deref, Component)]
pub struct Cloning(pub(super) Entity);

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

            let owner = self.owner(db);
            *db.get_mut::<Controller>(self.0).unwrap() = owner.into();

            db.entity_mut(self.0)
                .remove::<InLibrary>()
                .remove::<InHand>()
                .remove::<InStack>()
                .remove::<OnBattlefield>()
                .remove::<InGraveyard>()
                .remove::<InExile>()
                .insert(InHand(NEXT_HAND_SEQ.fetch_add(1, Ordering::Relaxed)));
        }
    }

    pub fn move_to_stack(self, db: &mut Database, targets: HashSet<ActiveTarget>) {
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

            let owner = self.owner(db);
            *db.get_mut::<Controller>(self.0).unwrap() = owner.into();

            db.entity_mut(self.0)
                .remove::<InLibrary>()
                .remove::<InHand>()
                .remove::<InStack>()
                .remove::<OnBattlefield>()
                .remove::<InGraveyard>()
                .remove::<InExile>()
                .insert(InStack(NEXT_STACK_SEQ.fetch_add(1, Ordering::Relaxed)))
                .insert(Targets(targets));
        }
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

            let owner = self.owner(db);
            *db.get_mut::<Controller>(self.0).unwrap() = owner.into();

            db.entity_mut(self.0)
                .remove::<InLibrary>()
                .remove::<InHand>()
                .remove::<InStack>()
                .remove::<OnBattlefield>()
                .remove::<InGraveyard>()
                .remove::<InExile>()
                .insert(InGraveyard(
                    NEXT_GRAVEYARD_SEQ.fetch_add(1, Ordering::Relaxed),
                ));
        }
    }

    pub fn move_to_library(self, db: &mut Database) {
        if self.is_token(db) {
            db.cards.despawn(self.0);
        } else {
            self.remove_all_modifiers(db);
            TriggerId::deactivate_all_for_card(db, self);
            ReplacementEffectId::deactivate_all_for_card(db, self);
            self.deactivate_modifiers(db);

            let owner = self.owner(db);
            *db.get_mut::<Controller>(self.0).unwrap() = owner.into();

            db.entity_mut(self.0)
                .remove::<InLibrary>()
                .remove::<InHand>()
                .remove::<InStack>()
                .remove::<OnBattlefield>()
                .remove::<InGraveyard>()
                .remove::<InExile>()
                .insert(InLibrary);
        }
    }

    pub fn move_to_exile(self, db: &mut Database) {
        if self.is_token(db) {
            db.cards.despawn(self.0);
        } else {
            self.remove_all_modifiers(db);
            TriggerId::deactivate_all_for_card(db, self);
            ReplacementEffectId::deactivate_all_for_card(db, self);
            self.deactivate_modifiers(db);

            let owner = self.owner(db);
            *db.get_mut::<Controller>(self.0).unwrap() = owner.into();

            db.entity_mut(self.0)
                .remove::<InLibrary>()
                .remove::<InHand>()
                .remove::<InStack>()
                .remove::<OnBattlefield>()
                .remove::<InGraveyard>()
                .remove::<InExile>()
                .insert(InExile);
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
                    Some(ModifierId(entity))
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
            ModifierId(entity).deactivate(db);
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
                    Some(ModifierId(entity))
                } else {
                    None
                }
            })
            .collect_vec();

        let reference: CardId = if let Some(cloning) = self.cloning(db) {
            cloning.0.into()
        } else {
            self
        };

        let facedown = self.facedown(db);

        let mut base_power = if facedown {
            Some(2)
        } else {
            db.get::<BasePower>(reference.0).map(|bp| bp.0)
        };
        let mut base_toughness = if facedown {
            Some(2)
        } else {
            db.get::<BaseToughness>(reference.0).map(|bt| bt.0)
        };
        let mut types = if facedown {
            HashSet::from([Type::Creature])
        } else {
            db.get::<Types>(reference.0).unwrap().0.clone()
        };
        let mut subtypes = if facedown {
            HashSet::default()
        } else {
            db.get::<Subtypes>(reference.0).unwrap().0.clone()
        };
        let mut keywords = if facedown {
            HashSet::default()
        } else {
            db.get::<Keywords>(reference.0).unwrap().0.clone()
        };
        let mut colors: HashSet<Color> = if facedown {
            HashSet::default()
        } else {
            db.get::<Colors>(reference.0)
                .unwrap()
                .0
                .union(&db.get::<CastingCost>(reference.0).unwrap().colors())
                .copied()
                .collect()
        };
        let mut triggers = if facedown {
            vec![]
        } else {
            db.get::<Triggers>(reference.0)
                .cloned()
                .map(|t| t.0)
                .unwrap_or_default()
        };
        let mut etb_abilities = if facedown {
            vec![]
        } else {
            db.get::<ETBAbilities>(reference.0)
                .cloned()
                .map(|t| t.0)
                .unwrap_or_default()
        };
        let mut static_abilities = if facedown {
            vec![]
        } else {
            db.get::<StaticAbilities>(reference.0)
                .cloned()
                .map(|s| s.0)
                .unwrap_or_default()
        };
        let mut activated_abilities = if facedown {
            vec![]
        } else {
            db.get::<ActivatedAbilities>(reference.0)
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
                let controller = self.controller(db);
                let controller_restriction = modifier.controller_restriction(db);
                let restrictions = modifier.restrictions(db);
                if !self.passes_restrictions_given_types(
                    db,
                    source,
                    controller,
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
        }

        for (ty, ability) in AbilityId::land_abilities(db) {
            if subtypes.contains(&ty) {
                activated_abilities.push(ability);
            }
        }

        for modifier in modifiers.iter().copied() {
            if !applied_modifiers.contains(&modifier) {
                let source = modifier.source(db);
                let controller = self.controller(db);
                let controller_restriction = modifier.controller_restriction(db);
                let restrictions = modifier.restrictions(db);
                if !self.passes_restrictions_given_types(
                    db,
                    source,
                    controller,
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
        }

        for modifier in modifiers.iter().copied() {
            if !applied_modifiers.contains(&modifier) {
                let source = modifier.source(db);
                let controller = self.controller(db);
                let controller_restriction = modifier.controller_restriction(db);
                let restrictions = modifier.restrictions(db);
                if !self.passes_restrictions_given_types(
                    db,
                    source,
                    controller,
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
                    EtbAbilityModifier::Add(add) => etb_abilities.push(add.clone()),
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
                    ModifyKeywords::Remove(remove) => keywords.retain(|kw| !remove.contains(kw)),
                    ModifyKeywords::Add(add) => keywords.extend(add.iter()),
                }
            }
        }

        let mut add_power = 0;
        let mut add_toughness = 0;

        for modifier in modifiers.iter().copied() {
            if !applied_modifiers.contains(&modifier) {
                let source = modifier.source(db);
                let controller = self.controller(db);
                let controller_restriction = modifier.controller_restriction(db);
                let restrictions = modifier.restrictions(db);
                if !self.passes_restrictions_given_types(
                    db,
                    source,
                    controller,
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

    pub fn etb_abilities(self, db: &mut Database) -> Vec<ETBAbility> {
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

    pub fn apply_modifier(self, db: &mut Database, modifier: ModifierId) {
        db.modifiers
            .get_mut::<Modifying>(modifier.0)
            .unwrap()
            .insert(self);
        modifier.activate(db);
        self.apply_modifiers_layered(db);
    }

    pub fn effects(self, db: &mut Database) -> Vec<AnyEffect> {
        db.get::<Effects>(self.0).cloned().unwrap_or_default().0
    }

    pub fn passes_restrictions(
        self,
        db: &mut Database,
        source: CardId,
        controller: Controller,
        controller_restriction: ControllerRestriction,
        restrictions: &[Restriction],
    ) -> bool {
        let types = self.types(db);
        let subtypes = self.subtypes(db);

        self.passes_restrictions_given_types(
            db,
            source,
            controller,
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
        controller: Controller,
        controller_restriction: ControllerRestriction,
        restrictions: &[Restriction],
        self_types: &HashSet<Type>,
        self_subtypes: &HashSet<Subtype>,
    ) -> bool {
        match controller_restriction {
            ControllerRestriction::Any => {}
            ControllerRestriction::You => {
                let source_controller = source.controller(db);
                if source_controller != controller {
                    return false;
                }
            }
            ControllerRestriction::Opponent => {
                let source_controller = source.controller(db);
                if source_controller == controller {
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
                    if controller.has_cards::<InHand>(db) {
                        return false;
                    }
                }
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

    pub fn power(self, db: &mut Database) -> Option<i32> {
        db.get::<ModifiedBasePower>(self.0)
            .map(|bp| bp.0)
            .or_else(|| db.get::<BasePower>(self.0).map(|bp| bp.0))
            .map(|bp| bp + db.get::<AddPower>(self.0).map(|a| a.0).unwrap_or_default())
    }

    pub fn toughness(self, db: &mut Database) -> Option<i32> {
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
            for mana in ability.cost().mana_cost.iter() {
                let color = mana.color();
                identity.insert(color);
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

    pub fn types_intersect(self, db: &mut Database, types: &HashSet<Type>) -> bool {
        types.is_empty() || !self.types(db).is_disjoint(types)
    }

    pub fn types(self, db: &mut Database) -> HashSet<Type> {
        db.get::<ModifiedTypes>(self.0)
            .map(|t| t.0.clone())
            .or_else(|| db.get::<Types>(self.0).map(|t| t.0.clone()))
            .unwrap_or_default()
    }

    pub fn subtypes_intersect(self, db: &mut Database, types: &HashSet<Subtype>) -> bool {
        types.is_empty() || !self.subtypes(db).is_disjoint(types)
    }

    pub fn subtypes(self, db: &mut Database) -> HashSet<Subtype> {
        db.get::<ModifiedSubtypes>(self.0)
            .map(|t| t.0.clone())
            .or_else(|| db.get::<Subtypes>(self.0).map(|t| t.0.clone()))
            .unwrap_or_default()
    }

    pub fn upload(db: &mut Database, cards: &Cards, player: Owner, name: &str) -> CardId {
        let card = cards.get(name).expect("Valid card name");

        Self::upload_card(db, card, player, InLibrary, false)
    }

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

    fn upload_card<Location: Component>(
        db: &mut Database,
        card: &Card,
        player: Owner,
        destination: Location,
        is_token: bool,
    ) -> CardId {
        let mut entity = db.spawn((
            destination,
            Name(card.name.clone()),
            player,
            Controller::from(player),
            card.cost.clone(),
            Types(card.types.clone()),
            Subtypes(card.subtypes.clone()),
            Colors(card.colors.clone()),
            Keywords(card.keywords.clone()),
        ));

        if is_token {
            entity.insert(IsToken);
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

        if card.split_second {
            entity.insert(SplitSecond);
        }

        if !card.effects.is_empty() {
            entity.insert(Effects(card.effects.clone()));
        }

        if !card.etb_abilities.is_empty() {
            entity.insert(ETBAbilities(card.etb_abilities.clone()));
        }

        let cardid = CardId(entity.id());

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
                let modifierid = upload_modifier(db, cardid, modifier, false);
                modifierids.push(modifierid);
            }

            let auraid = AuraId(
                db.auras
                    .spawn((
                        Modifiers(modifierids),
                        Restrictions(enchant.restrictions.clone()),
                    ))
                    .id(),
            );

            db.entity_mut(cardid.0).insert(auraid);
        }

        if !card.mana_gains.is_empty() {
            let mut ability_ids = vec![];
            for gain_mana in card.mana_gains.iter() {
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
                let mut entity = db.triggers.spawn((
                    TriggerListeners(HashSet::from([cardid])),
                    ability.trigger.from,
                    Effects(ability.effects.clone()),
                    Types(ability.trigger.for_types.clone()),
                ));

                match ability.trigger.trigger {
                    TriggerSource::PutIntoGraveyard => {
                        entity.insert(source::PutIntoGraveyard);
                    }
                    TriggerSource::EntersTheBattlefield => {
                        entity.insert(source::EntersTheBattlefield);
                    }
                }

                trigger_ids.push(TriggerId(entity.id()));
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
        cardid
    }

    pub fn cost(self, db: &mut Database) -> &CastingCost {
        db.get::<CastingCost>(self.0).unwrap()
    }

    pub fn valid_targets(self, db: &mut Database) -> HashSet<ActiveTarget> {
        let mut targets = HashSet::default();
        let creatures = Battlefield::creatures(db);

        for effect in self.effects(db) {
            self.targets_for_effect(db, effect, &mut targets, &creatures);
        }

        for ability in self.activated_abilities(db) {
            self.targets_for_ability(db, ability, &creatures, &mut targets);
        }

        targets
    }

    fn targets_for_damage(
        self,
        creatures: &[CardId],
        db: &mut Database,
        dmg: &DealDamage,
        targets: &mut HashSet<ActiveTarget>,
    ) {
        for creature in creatures.iter() {
            let controller = self.controller(db);
            if creature.can_be_targeted(db, controller)
                && creature.passes_restrictions(
                    db,
                    self,
                    controller,
                    ControllerRestriction::Any,
                    &dmg.restrictions,
                )
            {
                targets.insert(ActiveTarget::Battlefield { id: *creature });
            }
        }
    }

    pub fn targets_for_ability(
        self,
        db: &mut Database,
        ability: AbilityId,
        creatures: &[CardId],
        targets: &mut HashSet<ActiveTarget>,
    ) {
        let ability = ability.ability(db);
        if !ability.apply_to_self() {
            for effect in ability.into_effects() {
                self.targets_for_effect(db, effect, targets, creatures);
            }
        }
    }

    fn targets_for_effect(
        self,
        db: &mut Database,
        effect: AnyEffect,
        targets: &mut HashSet<ActiveTarget>,
        creatures: &[CardId],
    ) {
        let controller = self.controller(db);
        match effect.into_effect(db, controller) {
            Effect::CounterSpell { target } => {
                targets_for_counterspell(db, controller, &target, targets);
            }
            Effect::BattlefieldModifier(_) => {}
            Effect::ControllerDrawCards(_) => {}
            Effect::Equip(_) => {
                targets_for_battlefield_modifier(db, self, None, creatures, controller, targets);
            }
            Effect::CreateToken(_) => {}
            Effect::DealDamage(dmg) => {
                self.targets_for_damage(creatures, db, &dmg, targets);
            }
            Effect::ExileTargetCreature => {
                for creature in creatures.iter() {
                    if creature.can_be_targeted(db, controller) {
                        targets.insert(ActiveTarget::Battlefield { id: *creature });
                    }
                }
            }
            Effect::ExileTargetCreatureManifestTopOfLibrary => {
                for creature in creatures.iter() {
                    if creature.can_be_targeted(db, controller) {
                        targets.insert(ActiveTarget::Battlefield { id: *creature });
                    }
                }
            }
            Effect::GainCounter(_) => {}
            Effect::ModifyCreature(modifier) => {
                targets_for_battlefield_modifier(
                    db,
                    self,
                    Some(&modifier),
                    creatures,
                    controller,
                    targets,
                );
            }
            Effect::ControllerLosesLife(_) => {}
        }
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

    pub fn clone_card(&self, db: &mut Database, source: CardId) {
        db.entity_mut(self.0).insert(Cloning(source.0));
    }

    pub fn cloning(self, db: &mut Database) -> Option<Cloning> {
        db.get::<Cloning>(self.0).copied()
    }

    pub fn is_land(self, db: &mut Database) -> bool {
        self.types_intersect(db, &HashSet::from([Type::Land, Type::BasicLand]))
    }

    pub fn manifest(self, db: &mut Database) {
        db.entity_mut(self.0).insert(Manifested).insert(FaceDown);
    }

    pub fn is_permanent(self, db: &mut Database) -> bool {
        !self.types_intersect(db, &HashSet::from([Type::Instant, Type::Sorcery]))
    }

    pub fn keywords(self, db: &mut Database) -> HashSet<Keyword> {
        db.get::<ModifiedKeywords>(self.0)
            .map(|t| t.0.clone())
            .or_else(|| db.get::<Keywords>(self.0).map(|t| t.0.clone()))
            .unwrap_or_default()
    }

    pub fn shroud(self, db: &mut Database) -> bool {
        self.keywords(db).contains(&Keyword::Shroud)
    }

    pub fn hexproof(self, db: &mut Database) -> bool {
        self.keywords(db).contains(&Keyword::Hexproof)
    }

    pub fn flying(self, db: &mut Database) -> bool {
        self.keywords(db).contains(&Keyword::Flying)
    }

    pub fn vigilance(self, db: &mut Database) -> bool {
        self.keywords(db).contains(&Keyword::Vigilance)
    }

    pub fn name(self, db: &mut Database) -> String {
        db.get::<Name>(self.0).unwrap().0.clone()
    }

    pub fn split_second(self, db: &mut Database) -> bool {
        db.get::<SplitSecond>(self.0).is_some()
    }

    pub fn cannot_be_countered(&self, db: &mut Database) -> bool {
        db.get::<CannotBeCountered>(self.0).is_some()
    }
}