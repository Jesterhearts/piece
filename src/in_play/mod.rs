mod cardid;

use std::{
    cell::OnceCell,
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicUsize, Ordering},
};

use bevy_ecs::{component::Component, entity::Entity, query::With, world::World};
use derive_more::{Deref, DerefMut, From};
use itertools::Itertools;

use crate::{
    abilities::{
        Ability, ActivatedAbility, ApplyToSelf, GainMana, GainManaAbility, TriggerListeners,
    },
    card::{
        ActivatedAbilityModifier, AddColors, AddPower, AddToughness, BasePowerModifier,
        BaseToughnessModifier, EtbAbilityModifier, Keyword, ModifyKeywords, StaticAbilityModifier,
        TriggeredAbilityModifier,
    },
    controller::ControllerRestriction,
    cost::AbilityCost,
    effects::{
        counter,
        effect_duration::{UntilEndOfTurn, UntilSourceLeavesBattlefield},
        AnyEffect, BattlefieldModifier, Counter, DynamicPowerToughness, EffectDuration, Effects,
        ReplaceDraw, ReplacementEffect, Replacing,
    },
    mana::Mana,
    player::Controller,
    stack::{ActiveTarget, Stack, Targets},
    targets::{Restriction, Restrictions, SpellTarget},
    triggers::Location,
    types::{AddSubtypes, AddTypes, RemoveAllSubtypes, Subtype, Types},
};

pub use cardid::{CardId, Cloning};

static NEXT_BATTLEFIELD_SEQ: AtomicUsize = AtomicUsize::new(0);
static NEXT_GRAVEYARD_SEQ: AtomicUsize = AtomicUsize::new(0);
static NEXT_HAND_SEQ: AtomicUsize = AtomicUsize::new(0);
static NEXT_MODIFIER_SEQ: AtomicUsize = AtomicUsize::new(0);
static NEXT_REPLACEMENT_SEQ: AtomicUsize = AtomicUsize::new(0);
/// Starts at 1 because 0 should never be a valid stack id.
static NEXT_STACK_SEQ: AtomicUsize = AtomicUsize::new(1);

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

impl From<TriggerInStack> for InStack {
    fn from(value: TriggerInStack) -> Self {
        Self(value.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component, Hash)]
pub struct TriggerInStack(pub usize, pub CardId);

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

pub fn all_cards(db: &mut Database) -> Vec<CardId> {
    db.query::<Entity>().iter(db).map(CardId).collect()
}

pub fn cards<Location: Component + Ord>(db: &mut Database) -> Vec<CardId> {
    db.query::<(Entity, &Location)>()
        .iter(db)
        .sorted_by_key(|(_, loc)| *loc)
        .map(|(card, _)| CardId(card))
        .collect()
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
pub struct ReplacementDb(World);

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
    pub replacement_effects: ReplacementDb,
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

    pub fn controller(self, db: &mut Database) -> Controller {
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
            .insert(ModifyKeywords::Remove(Keyword::all()));
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

    fn add_types(self, db: &mut Database) -> Option<&AddTypes> {
        db.modifiers.get::<AddTypes>(self.0)
    }

    fn add_subtypes(self, db: &mut Database) -> Option<&AddSubtypes> {
        db.modifiers.get::<AddSubtypes>(self.0)
    }

    fn source(self, db: &mut Database) -> CardId {
        db.modifiers.get::<CardId>(self.0).copied().unwrap()
    }

    fn controller_restriction(self, db: &mut Database) -> ControllerRestriction {
        db.modifiers
            .get::<ControllerRestriction>(self.0)
            .copied()
            .unwrap()
    }

    fn restrictions(self, db: &mut Database) -> Vec<Restriction> {
        db.modifiers.get::<Restrictions>(self.0).cloned().unwrap().0
    }

    fn add_colors(self, db: &mut Database) -> Option<&AddColors> {
        db.modifiers.get::<AddColors>(self.0)
    }

    fn triggered_ability_modifiers(self, db: &mut Database) -> Option<&TriggeredAbilityModifier> {
        db.modifiers.get::<TriggeredAbilityModifier>(self.0)
    }

    fn etb_ability_modifiers(self, db: &mut Database) -> Option<&EtbAbilityModifier> {
        db.modifiers.get::<EtbAbilityModifier>(self.0)
    }

    fn static_ability_modifiers(self, db: &mut Database) -> Option<&StaticAbilityModifier> {
        db.modifiers.get::<StaticAbilityModifier>(self.0)
    }

    fn activated_ability_modifiers(self, db: &mut Database) -> Option<&ActivatedAbilityModifier> {
        db.modifiers.get::<ActivatedAbilityModifier>(self.0)
    }

    fn keyword_modifiers(self, db: &mut Database) -> Option<&ModifyKeywords> {
        db.modifiers.get::<ModifyKeywords>(self.0)
    }

    fn base_power(self, db: &mut Database) -> Option<i32> {
        db.modifiers.get::<BasePowerModifier>(self.0).map(|m| m.0)
    }

    fn base_toughness(self, db: &mut Database) -> Option<i32> {
        db.modifiers
            .get::<BaseToughnessModifier>(self.0)
            .map(|m| m.0)
    }

    fn add_power(self, db: &mut Database) -> Option<i32> {
        db.modifiers.get::<AddPower>(self.0).map(|a| a.0)
    }

    fn add_toughness(self, db: &mut Database) -> Option<i32> {
        db.modifiers.get::<AddToughness>(self.0).map(|a| a.0)
    }

    fn dynamic_power(self, db: &mut Database) -> Option<DynamicPowerToughness> {
        db.modifiers.get::<DynamicPowerToughness>(self.0).cloned()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, From)]
pub struct TriggerId(Entity);

impl TriggerId {
    pub fn move_to_stack(self, db: &mut Database, source: CardId, targets: HashSet<ActiveTarget>) {
        if Stack::split_second(db) {
            return;
        }

        db.triggers
            .entity_mut(self.0)
            .insert(TriggerInStack(
                NEXT_STACK_SEQ.fetch_add(1, Ordering::Relaxed),
                source,
            ))
            .insert(Targets(targets));
    }

    pub fn location_from(self, db: &mut Database) -> Location {
        db.triggers.get::<Location>(self.0).copied().unwrap()
    }

    pub fn for_types(self, db: &mut Database) -> Types {
        db.triggers.get::<Types>(self.0).cloned().unwrap()
    }

    pub fn listeners(self, db: &mut Database) -> HashSet<CardId> {
        db.triggers
            .get::<TriggerListeners>(self.0)
            .cloned()
            .map(|l| l.0)
            .unwrap()
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
            .query::<(Entity, &TriggerListeners)>()
            .iter(&db.triggers)
            .filter_map(|(entity, listeners)| {
                if listeners.contains(&cardid) {
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

    pub fn unsubscribe_all_for_card(db: &mut Database, cardid: CardId) {
        for mut listeners in db
            .triggers
            .query::<&mut TriggerListeners>()
            .iter_mut(&mut db.triggers)
        {
            listeners.remove(&cardid);
        }
    }

    pub fn deactivate_all_for_card(db: &mut Database, cardid: CardId) {
        let entities = db
            .triggers
            .query_filtered::<(Entity, &TriggerListeners), With<Active>>()
            .iter(&db.triggers)
            .filter_map(|(entity, listeners)| {
                if listeners.contains(&cardid) {
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

    fn add_listener(self, db: &mut Database, listener: CardId) {
        db.triggers
            .get_mut::<TriggerListeners>(self.0)
            .unwrap()
            .insert(listener);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component)]
pub struct ReplacementSeq(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplacementEffectId(Entity);

impl ReplacementEffectId {
    pub fn watching<Replacing: Component>(db: &mut Database) -> Vec<Self> {
        db.replacement_effects
            .query_filtered::<(Entity, &ReplacementSeq), (With<Active>, With<Replacing>)>()
            .iter(&db.replacement_effects)
            .sorted_by_key(|(_, seq)| *seq)
            .map(|(e, _)| Self(e))
            .collect_vec()
    }

    pub fn upload_replacement_effect(
        db: &mut Database,
        effect: &ReplacementEffect,
        source: CardId,
    ) -> Self {
        let mut entity = db.replacement_effects.spawn((
            source,
            Restrictions(effect.restrictions.clone()),
            Effects(effect.effects.clone()),
        ));

        match effect.replacing {
            Replacing::Draw => {
                entity.insert(ReplaceDraw);
            }
        }

        Self(entity.id())
    }

    pub fn activate_all_for_card(db: &mut Database, card: CardId) {
        let all = db
            .replacement_effects
            .query::<(Entity, &CardId)>()
            .iter(&db.replacement_effects)
            .filter_map(|(e, watcher)| if *watcher == card { Some(e) } else { None })
            .collect_vec();

        for entity in all {
            db.replacement_effects
                .entity_mut(entity)
                .insert(Active)
                .insert(ReplacementSeq(
                    NEXT_REPLACEMENT_SEQ.fetch_add(1, Ordering::Relaxed),
                ));
        }
    }

    pub fn deactivate_all_for_card(db: &mut Database, card: CardId) {
        let all = db
            .replacement_effects
            .query::<(Entity, &CardId)>()
            .iter(&db.replacement_effects)
            .filter_map(|(e, watcher)| if *watcher == card { Some(e) } else { None })
            .collect_vec();

        for entity in all {
            db.replacement_effects.entity_mut(entity).remove::<Active>();
        }
    }

    pub fn restrictions(self, db: &mut Database) -> Vec<Restriction> {
        db.replacement_effects
            .get::<Restrictions>(self.0)
            .unwrap()
            .0
            .clone()
    }

    pub fn effects(self, db: &mut Database) -> Vec<AnyEffect> {
        db.replacement_effects
            .get::<Effects>(self.0)
            .unwrap()
            .0
            .clone()
    }

    pub fn source(self, db: &mut Database) -> CardId {
        db.replacement_effects
            .get::<CardId>(self.0)
            .copied()
            .unwrap()
    }
}
