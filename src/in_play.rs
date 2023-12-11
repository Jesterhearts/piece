use std::{
    cell::OnceCell,
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicUsize, Ordering},
};

use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Or, With},
    world::World,
};
use derive_more::{Deref, DerefMut, From};
use itertools::Itertools;

use crate::{
    abilities::{
        Ability, ActivatedAbilities, ActivatedAbility, ApplyToSelf, ETBAbilities, GainMana,
        GainManaAbility, StaticAbilities, StaticAbility, Triggers,
    },
    battlefield::Battlefield,
    card::{
        ActivatedAbilityModifier, AddColors, AddHexproof, AddPower, AddShroud, AddToughness,
        AddVigilance, BasePower, BasePowerModifier, BaseToughness, BaseToughnessModifier,
        CannotBeCountered, Card, Color, Colors, Hexproof, MarkedDamage, Name, RemoveFlash,
        RemoveHexproof, RemoveShroud, RemoveVigilance, Shroud, SplitSecond, StaticAbilityModifier,
        TriggeredAbilities, TriggeredAbilityModifier, Vigilance,
    },
    controller::ControllerRestriction,
    cost::{AbilityCost, CastingCost},
    effects::{
        counter, AnyEffect, BattlefieldModifier, Counter, DealDamage, DynamicPowerToughness,
        Effect, EffectDuration, Effects, Token, UntilEndOfTurn, UntilSourceLeavesBattlefield,
    },
    mana::Mana,
    player::{Controller, Owner},
    stack::{ActiveTarget, Stack, Targets},
    targets::{Comparison, Restriction, Restrictions, SpellTarget},
    triggers::{source, Location, TriggerSource},
    types::{AddSubtypes, AddTypes, RemoveAllSubtypes, Subtype, Subtypes, Type, Types},
    Cards,
};

static NEXT_MODIFIER_SEQ: AtomicUsize = AtomicUsize::new(0);
/// Starts at 1 because 0 should never be a valid stack id.
static NEXT_STACK_SEQ: AtomicUsize = AtomicUsize::new(1);
static NEXT_GRAVEYARD_SEQ: AtomicUsize = AtomicUsize::new(0);
static NEXT_HAND_SEQ: AtomicUsize = AtomicUsize::new(0);
static NEXT_BATTLEFIELD_SEQ: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SequenceNumber(usize);

thread_local! {
    static INIT_LAND_ABILITIES: OnceCell<HashMap<Subtype, AbilityId>> = OnceCell::new();
}

#[derive(Debug, Component)]
pub struct Active;

#[derive(Debug, Component)]
pub struct Tapped;

#[derive(Debug, Component)]
pub struct Temporary;

#[derive(Debug, Clone, Copy, Component)]
pub struct Global;

#[derive(Debug, Clone, Copy, Component)]
pub struct EntireBattlefield;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component)]
pub struct InLibrary;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component)]
pub struct InHand(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Hash)]
pub struct InStack(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component)]
pub struct OnBattlefield(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component)]
pub struct InGraveyard(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component)]
pub struct InExile;

#[derive(Debug, Clone, Copy, Component)]
pub struct IsToken;

#[derive(Debug, Clone, Component, Deref, DerefMut, Default)]
pub struct Modifying(HashSet<CardId>);

#[derive(Debug, Clone, Copy, Component)]
pub struct FaceDown;

#[derive(Debug, Clone, Copy, Component)]
pub struct Manifested;

pub fn cards<Location: Component + Ord>(db: &mut Database) -> Vec<CardId> {
    db.query::<(Entity, &Location)>()
        .iter(db)
        .sorted_by_key(|(_, loc)| *loc)
        .map(|(card, _)| CardId(card))
        .collect()
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Deref, Component)]
pub struct CardId(Entity);

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From, Deref, Component)]
pub struct Cloning(Entity);

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

#[derive(Debug, Deref, DerefMut, Default)]
pub struct CardDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub struct ModifierDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub struct TriggerDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub struct ActivatedAbilityDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub struct StaticAbilityDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub struct AurasDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub struct CountersDb(World);

#[derive(Debug, Deref, DerefMut, Default)]
pub struct Database {
    #[deref]
    #[deref_mut]
    pub cards: CardDb,
    pub modifiers: ModifierDb,
    pub triggers: TriggerDb,
    pub abilities: ActivatedAbilityDb,
    pub static_abilities: StaticAbilityDb,
    pub auras: AurasDb,
    pub counters: CountersDb,
}

impl CardId {
    pub fn is_in_location<Location: Component + Ord>(self, world: &World) -> bool {
        world.get::<Location>(self.0).is_some()
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
    }

    pub fn move_to_graveyard(self, db: &mut Database) {
        if self.is_token(db) {
            db.cards.despawn(self.0);
        } else {
            self.remove_all_modifiers(db);
            TriggerId::deactivate_all_for_card(db, self);
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

    pub fn remove_modifier(self, db: &mut Database, modifier: ModifierId) {
        db.modifiers
            .get_mut::<Modifying>(modifier.0)
            .unwrap()
            .remove(&self);
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

    pub fn triggered_abilities(self, db: &mut Database) -> Vec<TriggerId> {
        let face_down = self.facedown(db);

        let mut triggers = if face_down {
            vec![]
        } else if let Some(cloning) = self.cloning(db) {
            db.get::<TriggeredAbilities>(cloning.0).unwrap().0.clone()
        } else {
            db.get::<TriggeredAbilities>(self.0).unwrap().0.clone()
        };

        let modifiers = db
            .modifiers
            .query_filtered::<(
                &TriggeredAbilityModifier,
                &CardId,
                &ControllerRestriction,
                &Restrictions,
                &ModifierSeq,
                &Modifying,
                Option<&Global>,
                Option<&EntireBattlefield>,
            ), With<Active>>()
            .iter(&db.modifiers)
            .sorted_by_key(|(_, _, _, _, active_seq, _, _, _)| *active_seq)
            .filter_map(
                |(
                    modifier,
                    source,
                    controller,
                    restrictions,
                    _,
                    modifying,
                    global,
                    entire_battlefield,
                )| {
                    if !(global.is_some()
                        || entire_battlefield.is_some()
                        || modifying.contains(&self))
                    {
                        None
                    } else {
                        Some((*modifier, *source, *controller, restrictions.clone()))
                    }
                },
            )
            .collect_vec();

        for (modifier, source, controller_restriction, restrictions) in modifiers {
            let controller = source.controller(db);
            if self.passes_restrictions(
                db,
                source,
                controller,
                controller_restriction,
                &restrictions,
            ) {
                match modifier {
                    TriggeredAbilityModifier::RemoveAll => triggers.clear(),
                    TriggeredAbilityModifier::Add(ability) => {
                        triggers.push(ability);
                    }
                }
            }
        }

        triggers
    }

    pub fn etb_abilities(self, db: &mut Database) -> Option<ETBAbilities> {
        if self.facedown(db) {
            return None;
        }

        if let Some(cloning) = self.cloning(db) {
            db.get::<ETBAbilities>(cloning.0).cloned()
        } else {
            db.get::<ETBAbilities>(self.0).cloned()
        }
    }

    pub fn static_abilities(self, db: &mut Database) -> Vec<StaticAbility> {
        let face_down = self.facedown(db);

        let mut abilities = if face_down {
            vec![]
        } else if let Some(cloning) = self.cloning(db) {
            db.get::<StaticAbilities>(cloning.0)
                .cloned()
                .unwrap_or_default()
                .0
        } else {
            db.get::<StaticAbilities>(self.0)
                .cloned()
                .unwrap_or_default()
                .0
        };

        let modifiers = db
            .modifiers
            .query_filtered::<(
                &StaticAbilityModifier,
                &CardId,
                &ControllerRestriction,
                &Restrictions,
                &ModifierSeq,
                &Modifying,
                Option<&Global>,
                Option<&EntireBattlefield>,
            ), With<Active>>()
            .iter(&db.modifiers)
            .sorted_by_key(|(_, _, _, _, active_seq, _, _, _)| *active_seq)
            .filter_map(
                |(
                    modifier,
                    source,
                    controller,
                    restrictions,
                    _,
                    modifying,
                    global,
                    entire_battlefield,
                )| {
                    if !(global.is_some()
                        || entire_battlefield.is_some()
                        || modifying.contains(&self))
                    {
                        None
                    } else {
                        Some((modifier.clone(), *source, *controller, restrictions.clone()))
                    }
                },
            )
            .collect_vec();

        for (modifier, source, controller_restriction, restrictions) in modifiers {
            let controller = source.controller(db);
            if self.passes_restrictions(
                db,
                source,
                controller,
                controller_restriction,
                &restrictions,
            ) {
                match modifier {
                    StaticAbilityModifier::RemoveAll => abilities.clear(),
                    StaticAbilityModifier::Add(ability) => {
                        abilities.push(ability);
                    }
                }
            }
        }

        abilities
    }
    pub fn activated_abilities(self, db: &mut Database) -> Vec<AbilityId> {
        let face_down = self.facedown(db);

        let mut abilities = if face_down {
            vec![]
        } else if let Some(cloning) = self.cloning(db) {
            db.get::<ActivatedAbilities>(cloning.0)
                .cloned()
                .unwrap_or_default()
                .0
        } else {
            db.get::<ActivatedAbilities>(self.0)
                .cloned()
                .unwrap_or_default()
                .0
        };

        let modifiers = db
            .modifiers
            .query_filtered::<(
                &ActivatedAbilityModifier,
                &CardId,
                &ControllerRestriction,
                &Restrictions,
                &ModifierSeq,
                &Modifying,
                Option<&Global>,
                Option<&EntireBattlefield>,
            ), With<Active>>()
            .iter(&db.modifiers)
            .sorted_by_key(|(_, _, _, _, active_seq, _, _, _)| *active_seq)
            .filter_map(
                |(
                    modifier,
                    source,
                    controller,
                    restrictions,
                    _,
                    modifying,
                    global,
                    entire_battlefield,
                )| {
                    if !(global.is_some()
                        || entire_battlefield.is_some()
                        || modifying.contains(&self))
                    {
                        None
                    } else {
                        Some((*modifier, *source, *controller, restrictions.clone()))
                    }
                },
            )
            .collect_vec();

        for (modifier, source, controller_restriction, restrictions) in modifiers {
            let controller = source.controller(db);
            if self.passes_restrictions(
                db,
                source,
                controller,
                controller_restriction,
                &restrictions,
            ) {
                match modifier {
                    ActivatedAbilityModifier::RemoveAll => abilities.clear(),
                    ActivatedAbilityModifier::Add(ability) => {
                        abilities.push(ability);
                    }
                }
            }
        }

        let land_abilities = AbilityId::land_abilities(db);
        for ty in self.subtypes(db) {
            if let Some(ability) = land_abilities.get(&ty) {
                abilities.push(*ability);
            }
        }

        abilities
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
        let mut base: Option<i32> = if self.facedown(db) {
            Some(2)
        } else if let Some(cloning) = self.cloning(db) {
            db.get::<BasePower>(cloning.0).map(|bp| bp.0)
        } else {
            db.get::<BasePower>(self.0).map(|bp| bp.0)
        };

        let modifiers = db
            .modifiers
            .query_filtered::<(
                Option<&BasePowerModifier>,
                Option<&DynamicPowerToughness>,
                Option<&AddPower>,
                &CardId,
                &ControllerRestriction,
                &Restrictions,
                &Modifying,
                Option<&Global>,
                Option<&EntireBattlefield>,
                &ModifierSeq,
            ), (
                With<Active>,
                Or<(&BasePowerModifier, &DynamicPowerToughness, &AddPower)>,
            )>()
            .iter(&db.modifiers)
            .sorted_by_key(|(_, _, _, _, _, _, _, _, _, seq)| *seq)
            .filter_map(
                |(
                    base,
                    dynamic,
                    add,
                    source,
                    restriction,
                    restrictions,
                    modifying,
                    global,
                    entire_battlefield,
                    _,
                )| {
                    if !(global.is_some()
                        || entire_battlefield.is_some()
                        || modifying.contains(&self))
                    {
                        None
                    } else {
                        Some((
                            base.copied(),
                            dynamic.copied(),
                            add.copied(),
                            *source,
                            *restriction,
                            restrictions.clone(),
                        ))
                    }
                },
            )
            .collect_vec();

        let mut add = 0;

        for (base_mod, dynamic_add_mod, add_mod, source, controller_restriction, restrictions) in
            modifiers
        {
            let controller = source.controller(db);
            if self.passes_restrictions(
                db,
                source,
                controller,
                controller_restriction,
                &restrictions,
            ) {
                if let Some(base_mod) = base_mod {
                    base = Some(base_mod.0);
                }
                add += *add_mod.unwrap_or_default();

                if let Some(dynamic) = dynamic_add_mod {
                    match dynamic {
                        DynamicPowerToughness::NumberOfCountersOnThis(counter) => {
                            let to_add = CounterId::counters_on(db, source, counter);
                            add += to_add as i32;
                        }
                    }
                }
            }
        }

        base.map(|base| base + add)
    }

    pub fn toughness(self, db: &mut Database) -> Option<i32> {
        let mut base: Option<i32> = if self.facedown(db) {
            Some(2)
        } else if let Some(cloning) = self.cloning(db) {
            db.get::<BaseToughness>(cloning.0).map(|bp| bp.0)
        } else {
            db.get::<BaseToughness>(self.0).map(|bp| bp.0)
        };

        let modifiers = db
            .modifiers
            .query_filtered::<(
                Option<&BaseToughnessModifier>,
                Option<&DynamicPowerToughness>,
                Option<&AddToughness>,
                &CardId,
                &ControllerRestriction,
                &Restrictions,
                &Modifying,
                Option<&Global>,
                Option<&EntireBattlefield>,
                &ModifierSeq,
            ), (
                With<Active>,
                Or<(
                    &BaseToughnessModifier,
                    &DynamicPowerToughness,
                    &AddToughness,
                )>,
            )>()
            .iter(&db.modifiers)
            .sorted_by_key(|(_, _, _, _, _, _, _, _, _, seq)| *seq)
            .filter_map(
                |(
                    base,
                    dynamic,
                    add,
                    source,
                    restriction,
                    restrictions,
                    modifying,
                    global,
                    entire_battlefield,
                    _,
                )| {
                    if !(global.is_some()
                        || entire_battlefield.is_some()
                        || modifying.contains(&self))
                    {
                        None
                    } else {
                        Some((
                            base.copied(),
                            dynamic.copied(),
                            add.copied(),
                            *source,
                            *restriction,
                            restrictions.clone(),
                        ))
                    }
                },
            )
            .collect_vec();

        let mut add = 0;

        for (base_mod, dynamic_add_mod, add_mod, source, controller_restriction, restrictions) in
            modifiers
        {
            let controller = source.controller(db);
            if self.passes_restrictions(
                db,
                source,
                controller,
                controller_restriction,
                &restrictions,
            ) {
                if let Some(base_mod) = base_mod {
                    base = Some(base_mod.0);
                }
                add += *add_mod.unwrap_or_default();

                if let Some(dynamic) = dynamic_add_mod {
                    match dynamic {
                        DynamicPowerToughness::NumberOfCountersOnThis(counter) => {
                            let to_add = CounterId::counters_on(db, source, counter);
                            add += to_add as i32;
                        }
                    }
                }
            }
        }

        base.map(|base| base + add)
    }

    pub fn aura(self, db: &mut Database) -> Option<AuraId> {
        db.get::<AuraId>(self.0).copied()
    }

    pub fn colors(self, db: &mut Database) -> HashSet<Color> {
        let mut colors = if let Some(cloning) = self.cloning(db) {
            db.get::<Colors>(cloning.0).unwrap().0.clone()
        } else {
            db.get::<Colors>(self.0).unwrap().0.clone()
        };

        let cost = if let Some(cloning) = self.cloning(db) {
            db.get::<CastingCost>(cloning.0).unwrap()
        } else {
            db.get::<CastingCost>(self.0).unwrap()
        };

        colors.extend(cost.colors());

        let modifiers = db
            .modifiers
            .query_filtered::<(
                &AddColors,
                &CardId,
                &ControllerRestriction,
                &Restrictions,
                &Modifying,
                Option<&Global>,
                Option<&EntireBattlefield>,
                &ModifierSeq,
            ), With<Active>>()
            .iter(&db.modifiers)
            .sorted_by_key(|(_, _, _, _, _, _, _, seq)| *seq)
            .filter_map(
                |(
                    add,
                    source,
                    restriction,
                    restrictions,
                    modifying,
                    global,
                    entire_battlefield,
                    _,
                )| {
                    if !(global.is_some()
                        || entire_battlefield.is_some()
                        || modifying.contains(&self))
                    {
                        None
                    } else {
                        Some((add.clone(), *source, *restriction, restrictions.clone()))
                    }
                },
            )
            .collect_vec();

        for (add, source, controller_restriction, restrictions) in modifiers {
            let controller = source.controller(db);
            if self.passes_restrictions(
                db,
                source,
                controller,
                controller_restriction,
                &restrictions,
            ) {
                colors.extend(add.iter());
            }
        }

        colors
    }

    pub fn color_identity(self, db: &mut Database) -> HashSet<Color> {
        let mut identity = db.get::<Colors>(self.0).unwrap().0.clone();

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
        let mut types = if let Some(cloning) = self.cloning(db) {
            db.get::<Types>(cloning.0).unwrap().0.clone()
        } else {
            db.get::<Types>(self.0).unwrap().0.clone()
        };

        let modifiers = db
            .modifiers
            .query_filtered::<(
                &AddTypes,
                &CardId,
                &ControllerRestriction,
                &Restrictions,
                &Modifying,
                Option<&Global>,
                Option<&EntireBattlefield>,
                &ModifierSeq,
            ), With<Active>>()
            .iter(&db.modifiers)
            .sorted_by_key(|(_, _, _, _, _, _, _, seq)| *seq)
            .filter_map(
                |(
                    add,
                    source,
                    restriction,
                    restrictions,
                    modifying,
                    global,
                    entire_battlefield,
                    _,
                )| {
                    if !(global.is_some()
                        || entire_battlefield.is_some()
                        || modifying.contains(&self))
                    {
                        None
                    } else {
                        Some((add.clone(), *source, *restriction, restrictions.clone()))
                    }
                },
            )
            .collect_vec();

        for (add, source, controller_restriction, restrictions) in modifiers {
            let controller = source.controller(db);
            let subtypes = self.subtypes(db);

            if self.passes_restrictions_given_types(
                db,
                source,
                controller,
                controller_restriction,
                &restrictions,
                &types,
                &subtypes,
            ) {
                types.extend(add.0.into_iter());
            }
        }

        types
    }

    pub fn subtypes_intersect(self, db: &mut Database, types: &HashSet<Subtype>) -> bool {
        types.is_empty() || !self.subtypes(db).is_disjoint(types)
    }

    pub fn subtypes(self, db: &mut Database) -> HashSet<Subtype> {
        let mut subtypes = if self.facedown(db) {
            HashSet::default()
        } else if let Some(cloning) = self.cloning(db) {
            db.get::<Subtypes>(cloning.0).unwrap().0.clone()
        } else {
            db.get::<Subtypes>(self.0).unwrap().0.clone()
        };

        let modifiers = db
            .modifiers
            .query_filtered::<(
                Option<&AddSubtypes>,
                Option<&RemoveAllSubtypes>,
                &CardId,
                &ControllerRestriction,
                &Restrictions,
                &Modifying,
                Option<&Global>,
                Option<&EntireBattlefield>,
                &ModifierSeq,
            ), (With<Active>, Or<(&AddSubtypes, &RemoveAllSubtypes)>)>()
            .iter(&db.modifiers)
            .sorted_by_key(|(_, _, _, _, _, _, _, _, seq)| *seq)
            .filter_map(
                |(
                    add,
                    remove,
                    source,
                    restriction,
                    restrictions,
                    modifying,
                    global,
                    entire_battlefield,
                    _,
                )| {
                    if !(global.is_some()
                        || entire_battlefield.is_some()
                        || modifying.contains(&self))
                    {
                        None
                    } else {
                        Some((
                            add.cloned(),
                            remove.copied(),
                            *source,
                            *restriction,
                            restrictions.clone(),
                        ))
                    }
                },
            )
            .collect_vec();

        for (add, remove, source, controller_restriction, restrictions) in modifiers {
            let controller = source.controller(db);
            let types = self.types(db);

            if self.passes_restrictions_given_types(
                db,
                source,
                controller,
                controller_restriction,
                &restrictions,
                &types,
                &subtypes,
            ) {
                if remove.is_some() {
                    subtypes.clear();
                }
                if let Some(add) = add {
                    subtypes.extend(add.0.into_iter());
                }
            }
        }

        subtypes
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
                    cardid,
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

        cardid
    }

    pub fn cost(self, db: &mut Database) -> &CastingCost {
        db.get::<CastingCost>(self.0).unwrap()
    }

    pub fn valid_targets(self, db: &mut Database) -> HashSet<ActiveTarget> {
        let mut targets = HashSet::default();
        let creatures = Battlefield::creatures(db);

        for effect in self.effects(db) {
            let controller = self.controller(db);
            let effect = effect.effect(db, controller);

            match effect {
                Effect::CounterSpell { target } => {
                    targets_for_counterspell(db, controller, target, &mut targets);
                }
                Effect::BattlefieldModifier(_) => {}
                Effect::ControllerDrawCards(_) => {}
                Effect::ModifyCreature(modifier) => {
                    targets_for_battlefield_modifier(
                        db,
                        self,
                        Some(modifier),
                        &creatures,
                        controller,
                        &mut targets,
                    );
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
                Effect::DealDamage(dmg) => {
                    self.targets_for_damage(&creatures, db, dmg, &mut targets);
                }
                Effect::CreateToken(_) => {}
                Effect::Equip(_) => {
                    targets_for_battlefield_modifier(
                        db,
                        self,
                        None,
                        &creatures,
                        controller,
                        &mut targets,
                    );
                }
                Effect::GainCounter(_) => {}
            }
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
                let controller = self.controller(db);
                match effect.into_effect(db, controller) {
                    Effect::CounterSpell { target } => {
                        targets_for_counterspell(db, controller, &target, targets);
                    }
                    Effect::BattlefieldModifier(_) => {}
                    Effect::ControllerDrawCards(_) => {}
                    Effect::Equip(_) => {
                        targets_for_battlefield_modifier(
                            db, self, None, creatures, controller, targets,
                        );
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
                }
            }
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

    pub fn vigilance(self, db: &mut Database) -> bool {
        let mut ability = if self.facedown(db) {
            false
        } else if let Some(cloning) = self.cloning(db) {
            db.get::<Vigilance>(cloning.0).is_some()
        } else {
            db.get::<Vigilance>(self.0).is_some()
        };

        let modifiers = db
            .modifiers
            .query_filtered::<(
                Option<&AddVigilance>,
                Option<&RemoveVigilance>,
                &CardId,
                &ControllerRestriction,
                &Restrictions,
                &Modifying,
                Option<&Global>,
                Option<&EntireBattlefield>,
                &ModifierSeq,
            ), (With<Active>, Or<(&AddVigilance, &RemoveVigilance)>)>()
            .iter(&db.modifiers)
            .sorted_by_key(|(_, _, _, _, _, _, _, _, seq)| *seq)
            .filter_map(
                |(
                    add,
                    remove,
                    source,
                    restriction,
                    restrictions,
                    modifying,
                    global,
                    entire_battlefield,
                    _,
                )| {
                    if !(global.is_some()
                        || entire_battlefield.is_some()
                        || modifying.contains(&self))
                    {
                        None
                    } else {
                        Some((
                            add.copied(),
                            remove.copied(),
                            *source,
                            *restriction,
                            restrictions.clone(),
                        ))
                    }
                },
            )
            .collect_vec();

        for (add, remove, source, controller_restriction, restrictions) in modifiers {
            let controller = source.controller(db);
            if self.passes_restrictions(
                db,
                source,
                controller,
                controller_restriction,
                &restrictions,
            ) {
                if remove.is_some() {
                    ability = false;
                }
                if add.is_some() {
                    ability = true;
                }
            }
        }

        ability
    }

    pub fn shroud(self, db: &mut Database) -> bool {
        let mut ability = if self.facedown(db) {
            false
        } else if let Some(cloning) = self.cloning(db) {
            db.get::<Shroud>(cloning.0).is_some()
        } else {
            db.get::<Shroud>(self.0).is_some()
        };

        let modifiers = db
            .modifiers
            .query_filtered::<(
                Option<&AddShroud>,
                Option<&RemoveShroud>,
                &CardId,
                &ControllerRestriction,
                &Restrictions,
                &Modifying,
                Option<&Global>,
                Option<&EntireBattlefield>,
                &ModifierSeq,
            ), (With<Active>, Or<(&AddShroud, &RemoveShroud)>)>()
            .iter(&db.modifiers)
            .sorted_by_key(|(_, _, _, _, _, _, _, _, seq)| *seq)
            .filter_map(
                |(
                    add,
                    remove,
                    source,
                    restriction,
                    restrictions,
                    modifying,
                    global,
                    entire_battlefield,
                    _,
                )| {
                    if !(global.is_some()
                        || entire_battlefield.is_some()
                        || modifying.contains(&self))
                    {
                        None
                    } else {
                        Some((
                            add.copied(),
                            remove.copied(),
                            *source,
                            *restriction,
                            restrictions.clone(),
                        ))
                    }
                },
            )
            .collect_vec();

        for (add, remove, source, controller_restriction, restrictions) in modifiers {
            let controller = source.controller(db);
            if self.passes_restrictions(
                db,
                source,
                controller,
                controller_restriction,
                &restrictions,
            ) {
                if remove.is_some() {
                    ability = false;
                }
                if add.is_some() {
                    ability = true;
                }
            }
        }

        ability
    }

    pub fn hexproof(self, db: &mut Database) -> bool {
        let mut ability = if self.facedown(db) {
            false
        } else if let Some(cloning) = self.cloning(db) {
            db.get::<Hexproof>(cloning.0).is_some()
        } else {
            db.get::<Hexproof>(self.0).is_some()
        };

        let modifiers = db
            .modifiers
            .query_filtered::<(
                Option<&AddHexproof>,
                Option<&RemoveHexproof>,
                &CardId,
                &ControllerRestriction,
                &Restrictions,
                &Modifying,
                Option<&Global>,
                Option<&EntireBattlefield>,
                &ModifierSeq,
            ), (With<Active>, Or<(&AddHexproof, &RemoveHexproof)>)>()
            .iter(&db.modifiers)
            .sorted_by_key(|(_, _, _, _, _, _, _, _, seq)| *seq)
            .filter_map(
                |(
                    add,
                    remove,
                    source,
                    restriction,
                    restrictions,
                    modifying,
                    global,
                    entire_battlefield,
                    _,
                )| {
                    if !(global.is_some()
                        || entire_battlefield.is_some()
                        || modifying.contains(&self))
                    {
                        None
                    } else {
                        Some((
                            add.copied(),
                            remove.copied(),
                            *source,
                            *restriction,
                            restrictions.clone(),
                        ))
                    }
                },
            )
            .collect_vec();

        for (add, remove, source, controller_restriction, restrictions) in modifiers {
            let controller = source.controller(db);
            if self.passes_restrictions(
                db,
                source,
                controller,
                controller_restriction,
                &restrictions,
            ) {
                if remove.is_some() {
                    ability = false;
                }
                if add.is_some() {
                    ability = true;
                }
            }
        }

        ability
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

    pub(crate) fn manifest(self, db: &mut Database) {
        db.entity_mut(self.0).insert(Manifested).insert(FaceDown);
    }

    pub fn is_permanent(self, db: &mut Database) -> bool {
        !self.types_intersect(db, &HashSet::from([Type::Instant, Type::Sorcery]))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Component)]
pub struct AuraId(Entity);

impl AuraId {
    pub fn modifiers(self, db: &mut Database) -> Modifiers {
        db.auras.get::<Modifiers>(self.0).cloned().unwrap()
    }

    pub fn is_attached(self, db: &mut Database) -> bool {
        let modifiers = self.modifiers(db);
        for modifier in modifiers.iter() {
            if !modifier.modifying(db).is_empty() {
                return true;
            }
        }

        false
    }
}

fn upload_modifier(
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

    if modifier.modifier.remove_all_subtypes {
        entity.insert(RemoveAllSubtypes);
    }

    if modifier.modifier.add_vigilance {
        entity.insert(AddVigilance);
    }

    let modifierid = ModifierId(entity.id());

    if let Some(ability) = &modifier.modifier.add_ability {
        let id = AbilityId::upload_ability(db, source, Ability::Activated(ability.clone()));
        db.modifiers
            .entity_mut(modifierid.0)
            .insert(ActivatedAbilityModifier::Add(id));
    }

    if let Some(ability) = &modifier.modifier.gain_mana {
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

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From)]
pub struct AbilityId(Entity);

impl AbilityId {
    pub fn upload_ability(db: &mut Database, cardid: CardId, ability: Ability) -> AbilityId {
        match ability {
            Ability::Activated(ability) => {
                let mut entity =
                    db.abilities
                        .spawn((cardid, ability.cost, Effects(ability.effects)));

                if ability.apply_to_self {
                    entity.insert(ApplyToSelf);
                }

                Self(entity.id())
            }
            Ability::Mana(ability) => {
                let entity = db.abilities.spawn((cardid, ability.cost, ability.gain));

                Self(entity.id())
            }
        }
    }

    pub fn land_abilities(db: &mut Database) -> HashMap<Subtype, Self> {
        INIT_LAND_ABILITIES.with(|init| {
            init.get_or_init(|| {
                let mut abilities = HashMap::new();

                let id = AbilityId(
                    db.abilities
                        .spawn((
                            AbilityCost {
                                mana_cost: vec![],
                                tap: true,
                                additional_cost: vec![],
                            },
                            GainMana::Specific {
                                gains: vec![Mana::White],
                            },
                        ))
                        .id(),
                );
                abilities.insert(Subtype::Plains, id);

                let id = AbilityId(
                    db.abilities
                        .spawn((
                            AbilityCost {
                                mana_cost: vec![],
                                tap: true,
                                additional_cost: vec![],
                            },
                            GainMana::Specific {
                                gains: vec![Mana::Blue],
                            },
                        ))
                        .id(),
                );
                abilities.insert(Subtype::Island, id);

                let id = AbilityId(
                    db.abilities
                        .spawn((
                            AbilityCost {
                                mana_cost: vec![],
                                tap: true,
                                additional_cost: vec![],
                            },
                            GainMana::Specific {
                                gains: vec![Mana::Black],
                            },
                        ))
                        .id(),
                );
                abilities.insert(Subtype::Swamp, id);

                let id = AbilityId(
                    db.abilities
                        .spawn((
                            AbilityCost {
                                mana_cost: vec![],
                                tap: true,
                                additional_cost: vec![],
                            },
                            GainMana::Specific {
                                gains: vec![Mana::Red],
                            },
                        ))
                        .id(),
                );
                abilities.insert(Subtype::Mountain, id);

                let id = AbilityId(
                    db.abilities
                        .spawn((
                            AbilityCost {
                                mana_cost: vec![],
                                tap: true,
                                additional_cost: vec![],
                            },
                            GainMana::Specific {
                                gains: vec![Mana::Green],
                            },
                        ))
                        .id(),
                );
                abilities.insert(Subtype::Forest, id);

                abilities
            })
            .clone()
        })
    }

    pub fn move_to_stack(self, db: &mut Database, source: CardId, targets: HashSet<ActiveTarget>) {
        if Stack::split_second(db) {
            return;
        }

        db.abilities
            .entity_mut(self.0)
            .insert(InStack(NEXT_STACK_SEQ.fetch_add(1, Ordering::Relaxed)))
            .insert(Targets(targets))
            // This is a hack to make land types work, probably need something better here
            .insert(source);
    }

    pub fn ability(self, db: &mut Database) -> Ability {
        if let Some((cost, effects, apply_to_self)) = db
            .abilities
            .query::<(Entity, &AbilityCost, &Effects, Option<&ApplyToSelf>)>()
            .iter(&db.abilities)
            .filter_map(|(e, cost, effect, apply_to_self)| {
                if Self(e) == self {
                    Some((cost, effect, apply_to_self))
                } else {
                    None
                }
            })
            .next()
        {
            Ability::Activated(ActivatedAbility {
                cost: cost.clone(),
                effects: effects.0.clone(),
                apply_to_self: apply_to_self.is_some(),
            })
        } else {
            Ability::Mana(self.gain_mana_ability(db))
        }
    }

    pub fn gain_mana_ability(self, db: &mut Database) -> GainManaAbility {
        let (cost, gain) = db
            .abilities
            .query::<(Entity, &AbilityCost, &GainMana)>()
            .iter(&db.abilities)
            .filter_map(|(e, cost, effect)| {
                if Self(e) == self {
                    Some((cost, effect))
                } else {
                    None
                }
            })
            .next()
            .unwrap();

        GainManaAbility {
            cost: cost.clone(),
            gain: gain.clone(),
        }
    }

    pub fn apply_to_self(self, db: &mut Database) -> bool {
        db.abilities.get::<ApplyToSelf>(self.0).is_some()
    }

    pub fn effects(self, db: &mut Database) -> Vec<AnyEffect> {
        db.abilities
            .get::<Effects>(self.0)
            .cloned()
            .unwrap_or_default()
            .0
    }

    pub fn source(self, db: &mut Database) -> CardId {
        db.abilities.get::<CardId>(self.0).copied().unwrap()
    }

    pub(crate) fn controller(self, db: &mut Database) -> Controller {
        self.source(db).controller(db)
    }

    fn delete(self, db: &mut Database) {
        db.abilities.despawn(self.0);
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord, Component)]
pub struct ModifierSeq(usize);

impl ModifierSeq {
    pub fn next() -> Self {
        Self(NEXT_MODIFIER_SEQ.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Default, Component, Deref, DerefMut)]
pub struct Modifiers(pub Vec<ModifierId>);

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From)]
pub struct ModifierId(Entity);

impl ModifierId {
    pub fn remove_all_abilities(self, db: &mut Database) {
        db.modifiers
            .entity_mut(self.0)
            .insert(ActivatedAbilityModifier::RemoveAll)
            .insert(StaticAbilityModifier::RemoveAll)
            .insert(TriggeredAbilityModifier::RemoveAll)
            .insert(RemoveVigilance)
            .insert(RemoveFlash)
            .insert(RemoveHexproof)
            .insert(RemoveShroud);
    }

    pub fn upload_temporary_modifier(
        db: &mut Database,
        cardid: CardId,
        modifier: &BattlefieldModifier,
    ) -> ModifierId {
        upload_modifier(db, cardid, modifier, true)
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
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From)]
pub struct TriggerId(Entity);

impl TriggerId {
    pub fn move_to_stack(self, db: &mut Database, targets: HashSet<ActiveTarget>) {
        if Stack::split_second(db) {
            return;
        }

        db.triggers
            .entity_mut(self.0)
            .insert(InStack(NEXT_STACK_SEQ.fetch_add(1, Ordering::Relaxed)))
            .insert(Targets(targets));
    }

    pub fn location_from(self, db: &mut Database) -> Location {
        db.triggers.get::<Location>(self.0).copied().unwrap()
    }

    pub fn for_types(self, db: &mut Database) -> Types {
        db.triggers.get::<Types>(self.0).cloned().unwrap()
    }

    pub fn listener(self, db: &mut Database) -> CardId {
        db.triggers.get::<CardId>(self.0).copied().unwrap()
    }

    pub fn effects(self, db: &mut Database) -> Vec<AnyEffect> {
        db.triggers
            .get::<Effects>(self.0)
            .cloned()
            .unwrap_or_default()
            .0
    }

    pub fn active_triggers_of_source<Source: Component>(db: &mut Database) -> Vec<TriggerId> {
        let mut results = vec![];
        let mut of_type = db
            .triggers
            .query_filtered::<Entity, (With<Source>, With<Active>)>();

        for id in of_type.iter(&db.triggers) {
            results.push(Self(id));
        }

        results
    }

    pub fn activate_all_for_card(db: &mut Database, cardid: CardId) {
        let entities = db
            .triggers
            .query::<(Entity, &CardId)>()
            .iter(&db.triggers)
            .filter_map(|(entity, source)| {
                if cardid == *source {
                    Some(entity)
                } else {
                    None
                }
            })
            .collect_vec();

        for entity in entities {
            db.triggers.entity_mut(entity).insert(Active);
        }
    }

    pub fn deactivate_all_for_card(db: &mut Database, cardid: CardId) {
        let entities = db
            .triggers
            .query_filtered::<(Entity, &CardId), With<Active>>()
            .iter(&db.triggers)
            .filter_map(|(entity, listener)| {
                if *listener == cardid {
                    Some(entity)
                } else {
                    None
                }
            })
            .collect_vec();

        for entity in entities {
            db.triggers.entity_mut(entity).remove::<Active>();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deref, DerefMut, Component)]
pub struct Count(pub usize);

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct CounterId(Entity);

impl CounterId {
    pub fn add_counters(db: &mut Database, card: CardId, counter: Counter, count: usize) {
        match counter {
            Counter::Charge => Self::add_counters_of_type::<counter::Charge>(db, card, count),
            Counter::P1P1 => Self::add_counters_of_type::<counter::P1P1>(db, card, count),
            Counter::M1M1 => Self::add_counters_of_type::<counter::M1M1>(db, card, count),
        }
    }

    pub fn add_counters_of_type<Type: Component + Default>(
        db: &mut Database,
        card: CardId,
        count: usize,
    ) {
        let existing = db
            .counters
            .query_filtered::<(&CardId, &mut Count), With<Type>>()
            .iter_mut(&mut db.counters)
            .find_map(
                |(is_on, count)| {
                    if card == *is_on {
                        Some(count)
                    } else {
                        None
                    }
                },
            );

        if let Some(mut existing_count) = existing {
            **existing_count += count;
        } else {
            db.counters.spawn((card, Count(count), Type::default()));
        }
    }

    pub fn counters_on(db: &mut Database, card: CardId, counter: Counter) -> usize {
        match counter {
            Counter::Charge => Self::counters_of_type_on::<counter::Charge>(db, card),
            Counter::P1P1 => Self::counters_of_type_on::<counter::P1P1>(db, card),
            Counter::M1M1 => Self::counters_of_type_on::<counter::M1M1>(db, card),
        }
    }

    pub fn counters_of_type_on<Type: Component>(db: &mut Database, card: CardId) -> usize {
        db.counters
            .query_filtered::<(&CardId, &Count), With<Type>>()
            .iter_mut(&mut db.counters)
            .find_map(
                |(is_on, count)| {
                    if card == *is_on {
                        Some(**count)
                    } else {
                        None
                    }
                },
            )
            .unwrap_or_default()
    }
}

fn targets_for_counterspell(
    db: &mut Database,
    caster: Controller,
    target: &SpellTarget,
    targets: &mut HashSet<ActiveTarget>,
) {
    let cards_in_stack = db
        .query::<(Entity, &InStack)>()
        .iter(db)
        .map(|(entity, in_stack)| (CardId(entity), *in_stack))
        .sorted_by_key(|(_, in_stack)| *in_stack)
        .collect_vec();

    for (card, stack_id) in cards_in_stack {
        if card.can_be_countered(db, caster, target) {
            targets.insert(ActiveTarget::Stack { id: stack_id });
        }
    }
}

fn targets_for_battlefield_modifier(
    db: &mut Database,
    source: CardId,
    modifier: Option<&BattlefieldModifier>,
    creatures: &[CardId],
    caster: Controller,
    targets: &mut HashSet<ActiveTarget>,
) {
    for creature in creatures.iter() {
        if creature.can_be_targeted(db, caster)
            && (modifier.is_none()
                || creature.passes_restrictions(
                    db,
                    source,
                    caster,
                    modifier.unwrap().controller,
                    &modifier.unwrap().restrictions,
                ))
        {
            targets.insert(ActiveTarget::Battlefield { id: *creature });
        }
    }
}
