use std::collections::{HashMap, HashSet};

use indexmap::IndexSet;
use itertools::Itertools;
use protobuf::Enum;
use strum::IntoEnumIterator;
use tracing::Level;
use uuid::Uuid;

use crate::{
    abilities::Ability,
    battlefield::Battlefields,
    card::{BasePowerType, BaseToughnessType},
    effects::EffectBehaviors,
    in_play::{
        ActivatedAbilityId, CastFrom, Database, ExileReason, GainManaAbilityId, ModifierId,
        StaticAbilityId,
    },
    log::{LeaveReason, Log, LogEntry, LogId},
    pending_results::PendingResults,
    player::{Controller, Owner},
    protogen::{
        abilities::TriggeredAbility,
        card::Card,
        color::Color,
        cost::CastingCost,
        counters::Counter,
        effects::{
            create_token::Token,
            dynamic_power_toughness,
            replacement_effect::Replacing,
            static_ability::{
                self, AddKeywordsIf, AllAbilitiesOfExiledWith, GreenCannotBeCountered,
            },
            Duration, DynamicPowerToughness, Effect, ReplacementEffect,
        },
        keywords::Keyword,
        mana::ManaSource,
        targets::{
            comparison,
            dynamic::Dynamic,
            restriction::{
                self, cmc::Cmc, EnteredBattlefieldThisTurn, NotOfType, NumberOfCountersOnThis,
                OfColor, OfType,
            },
            Location, Restriction,
        },
        triggers::TriggerSource,
        types::{Subtype, Type},
    },
    stack::{ActiveTarget, Stack},
    types::{SubtypeSet, TypeSet},
    Cards,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct CardId(Uuid);

impl std::fmt::Display for CardId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

#[derive(Debug, Default)]
pub struct CardInPlay {
    pub card: Card,
    pub cloning: Option<Card>,
    pub(crate) cloned_id: Option<CardId>,

    pub(crate) static_abilities: HashSet<StaticAbilityId>,
    pub(crate) activated_abilities: IndexSet<ActivatedAbilityId>,
    pub(crate) mana_abilities: IndexSet<GainManaAbilityId>,

    pub(crate) owner: Owner,
    pub(crate) controller: Controller,

    pub(crate) entered_battlefield_turn: Option<usize>,
    pub(crate) left_battlefield_turn: Option<usize>,

    pub(crate) cast_from: Option<CastFrom>,

    pub(crate) exiling: HashSet<CardId>,
    pub(crate) exile_reason: Option<ExileReason>,
    pub(crate) exile_duration: Option<Duration>,

    pub(crate) sourced_mana: HashMap<ManaSource, usize>,

    pub(crate) x_is: usize,

    pub(crate) enchanting: Option<CardId>,
    pub(crate) revealed: bool,
    pub(crate) tapped: bool,
    pub(crate) attacking: Option<Owner>,
    pub manifested: bool,
    pub(crate) facedown: bool,
    pub(crate) transformed: bool,
    pub(crate) token: bool,

    pub(crate) replacements_active: bool,

    pub modified_name: String,
    pub modified_cost: CastingCost,
    pub(crate) modified_base_power: Option<BasePowerType>,
    pub(crate) modified_base_toughness: Option<BaseToughnessType>,
    pub(crate) add_power: i32,
    pub(crate) add_toughness: i32,
    pub modified_types: TypeSet,
    pub modified_subtypes: SubtypeSet,
    pub(crate) modified_colors: HashSet<Color>,
    pub modified_keywords: HashMap<i32, u32>,
    pub(crate) modified_replacement_abilities: HashMap<Replacing, Vec<ReplacementEffect>>,
    pub modified_triggers: HashMap<TriggerSource, Vec<TriggeredAbility>>,
    pub modified_etb_abilities: Vec<Effect>,
    pub(crate) modified_static_abilities: HashSet<StaticAbilityId>,
    pub(crate) modified_activated_abilities: IndexSet<ActivatedAbilityId>,
    pub(crate) modified_mana_abilities: IndexSet<GainManaAbilityId>,

    pub(crate) marked_damage: i32,

    pub(crate) counters: HashMap<Counter, usize>,
}

impl CardInPlay {
    fn reset(&mut self, preserve_exiled: bool) {
        let mut card = Card::default();
        std::mem::swap(&mut card, &mut self.card);

        let mut static_abilities = HashSet::default();
        std::mem::swap(&mut static_abilities, &mut self.static_abilities);

        let mut activated_abilities = IndexSet::default();
        std::mem::swap(&mut activated_abilities, &mut self.activated_abilities);

        let mut mana_abilities = IndexSet::default();
        std::mem::swap(&mut mana_abilities, &mut self.mana_abilities);

        let mut exiling = HashSet::default();
        if preserve_exiled {
            std::mem::swap(&mut exiling, &mut self.exiling);
        }

        let owner = self.owner;
        *self = Self {
            card,
            owner,
            static_abilities,
            activated_abilities,
            mana_abilities,
            controller: owner.into(),
            exiling,
            ..Default::default()
        };
    }

    pub fn abilities(&self, db: &Database) -> Vec<(CardId, Ability)> {
        self.modified_mana_abilities
            .iter()
            .map(|ability| (db[*ability].source, Ability::Mana(*ability)))
            .chain(
                self.modified_activated_abilities
                    .iter()
                    .map(|ability| (db[*ability].source, Ability::Activated(*ability))),
            )
            .collect_vec()
    }

    pub fn counter_text_on(&self) -> Vec<String> {
        let mut results = vec![];

        for counter in Counter::iter() {
            let amount = self.counters.get(&counter).copied().unwrap_or_default();
            if amount > 0 {
                results.push(match counter {
                    Counter::CHARGE => format!("Charge x{}", amount),
                    Counter::NET => format!("Net x{}", amount),
                    Counter::P1P1 => format!("+1/+1 x{}", amount),
                    Counter::M1M1 => format!("-1/-1 x{}", amount),
                    Counter::ANY => format!("{} total counters", amount),
                });
            }
        }

        results
    }
}

impl CardId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn upload(db: &mut Database, cards: &Cards, player: Owner, card: &str) -> CardId {
        let card = cards.get(card).expect("Invalid card name");
        let id = Self::new();

        let mut static_abilities = HashSet::default();
        for ability in card.static_abilities.iter() {
            static_abilities.insert(StaticAbilityId::upload(
                db,
                id,
                ability.ability.as_ref().unwrap().clone(),
            ));
        }

        let mut activated_abilities = IndexSet::default();
        for ability in card.activated_abilities.iter() {
            activated_abilities.insert(ActivatedAbilityId::upload(db, id, ability.clone()));
        }

        let mut mana_abilities = IndexSet::default();
        for ability in card.mana_abilities.iter() {
            mana_abilities.insert(GainManaAbilityId::upload(db, id, ability.clone()));
        }

        db.cards.insert(
            id,
            CardInPlay {
                card: card.clone(),
                controller: player.into(),
                owner: player,
                static_abilities,
                activated_abilities,
                mana_abilities,
                ..Default::default()
            },
        );

        id.apply_modifiers_layered(db);
        id
    }

    pub(crate) fn upload_token(db: &mut Database, player: Owner, token: Token) -> CardId {
        let id = Self::new();
        db.cards.insert(
            id,
            CardInPlay {
                card: token.into(),
                controller: player.into(),
                owner: player,
                token: true,
                ..Default::default()
            },
        );

        id.apply_modifiers_layered(db);
        id
    }

    pub fn is_in_location(self, db: &Database, location: Location) -> bool {
        match location {
            Location::ON_BATTLEFIELD => db.battlefield[db[self].controller].contains(&self),
            Location::IN_GRAVEYARD => db.graveyard[db[self].owner].contains(&self),
            Location::IN_EXILE => db.exile[db[self].owner].contains(&self),
            Location::IN_LIBRARY => db.all_players[db[self].owner].library.cards.contains(&self),
            Location::IN_HAND => db.hand[db[self].owner].contains(&self),
            Location::IN_STACK => db.stack.contains(self),
        }
    }

    pub(crate) fn transform(self, db: &mut Database) {
        db[self].facedown = !db[self].facedown;
        db[self].transformed = !db[self].transformed;

        db[self].static_abilities.clear();
        db[self].activated_abilities.clear();
        db[self].mana_abilities.clear();

        for ability in self.faceup_face(db).static_abilities.clone() {
            let id = StaticAbilityId::upload(db, self, ability.ability.unwrap().clone());
            db[self].static_abilities.insert(id);
        }

        for ability in self.faceup_face(db).activated_abilities.clone() {
            let id = ActivatedAbilityId::upload(db, self, ability);
            db[self].activated_abilities.insert(id);
        }

        for ability in self.faceup_face(db).mana_abilities.clone() {
            let id = GainManaAbilityId::upload(db, self, ability);
            db[self].mana_abilities.insert(id);
        }

        self.apply_modifiers_layered(db);
    }

    pub fn faceup_face(self, db: &Database) -> &Card {
        if let Some(cloning) = db[self].cloning.as_ref() {
            cloning
        } else if db[self].facedown {
            db[self].card.back_face.as_ref().unwrap_or(&db[self].card)
        } else {
            &db[self].card
        }
    }

    pub(crate) fn entered_battlefield_this_turn(
        db: &Database,
    ) -> impl Iterator<Item = CardId> + '_ {
        db.cards.iter().filter_map(|(id, card)| {
            if card.entered_battlefield_turn == Some(db.turn.turn_count) {
                Some(*id)
            } else {
                None
            }
        })
    }

    pub(crate) fn left_battlefield_this_turn(db: &Database) -> impl Iterator<Item = CardId> + '_ {
        db.cards.iter().filter_map(|(id, card)| {
            if card.left_battlefield_turn == Some(db.turn.turn_count) {
                Some(*id)
            } else {
                None
            }
        })
    }

    pub fn move_to_hand(self, db: &mut Database) {
        if self.is_in_location(db, Location::ON_BATTLEFIELD) {
            Log::left_battlefield(db, LeaveReason::ReturnedToHand, self);
        }

        if db[self].token {
            self.move_to_limbo(db);
        } else {
            self.remove_all_modifiers(db);

            db[self].reset(false);
            db.stack.remove(self);

            let view = db.owner_view_mut(db[self].owner);
            view.battlefield.shift_remove(&self);
            view.graveyard.shift_remove(&self);
            view.exile.shift_remove(&self);
            view.library.remove(self);

            view.hand.insert(self);

            for sa in db[self]
                .modified_static_abilities
                .clone()
                .into_iter()
                .collect_vec()
            {
                if let Some(modifier) = db[sa].owned_modifier.take() {
                    modifier.deactivate(db);
                }
            }

            self.apply_modifiers_layered(db);
        }
    }

    pub(crate) fn move_to_stack(
        self,
        db: &mut Database,
        targets: Vec<Vec<ActiveTarget>>,
        from: Option<CastFrom>,
        chosen_modes: Vec<usize>,
    ) -> PendingResults {
        if db.stack.split_second(db) {
            warn!("Skipping add to stack (split second)");
            return PendingResults::default();
        }

        if db[self].token {
            self.move_to_limbo(db);
            PendingResults::default()
        } else {
            self.remove_all_modifiers(db);

            db[self].replacements_active = false;
            db[self].cast_from = from;

            let view = db.owner_view_mut(db[self].owner);
            view.battlefield.shift_remove(&self);
            view.graveyard.shift_remove(&self);
            view.exile.shift_remove(&self);
            view.library.remove(self);
            view.hand.shift_remove(&self);

            Stack::push_card(db, self, targets, chosen_modes)
        }
    }

    pub(crate) fn move_to_battlefield(self, db: &mut Database) {
        db.stack.remove(self);

        let view = db.owner_view_mut(db[self].controller.into());
        view.graveyard.shift_remove(&self);
        view.exile.shift_remove(&self);
        view.library.remove(self);
        view.hand.shift_remove(&self);

        view.battlefield.insert(self);

        for modifier in db[self]
            .modified_static_abilities
            .iter()
            .filter_map(|sa| db[*sa].owned_modifier)
            .collect_vec()
        {
            modifier.activate(&mut db.modifiers);
        }

        db[self].entered_battlefield_turn = Some(db.turn.turn_count);

        self.apply_modifiers_layered(db);
    }

    pub(crate) fn move_to_graveyard(self, db: &mut Database) {
        if self.is_in_location(db, Location::ON_BATTLEFIELD) {
            Log::left_battlefield(db, LeaveReason::PutIntoGraveyard, self);
        }

        if db[self].token {
            self.move_to_limbo(db);
        } else {
            self.remove_all_modifiers(db);

            db[self].reset(false);
            db.stack.remove(self);
            let view = db.owner_view_mut(db[self].owner);
            view.exile.shift_remove(&self);
            view.library.remove(self);
            view.hand.shift_remove(&self);
            view.battlefield.shift_remove(&self);

            let owner = db[self].owner;
            db.graveyard[owner].insert(self);
            if self.is_permanent(db) {
                *db.graveyard.descended_this_turn.entry(owner).or_default() += 1;
            }

            for sa in db[self]
                .modified_static_abilities
                .clone()
                .into_iter()
                .collect_vec()
            {
                if let Some(modifier) = db[sa].owned_modifier.take() {
                    modifier.deactivate(db);
                }
            }

            self.apply_modifiers_layered(db);
        }
    }

    pub(crate) fn move_to_library(self, db: &mut Database) -> bool {
        if self.is_in_location(db, Location::ON_BATTLEFIELD) {
            Log::left_battlefield(db, LeaveReason::ReturnedToLibrary, self);
        }

        if db[self].token {
            self.move_to_limbo(db);
            false
        } else {
            self.remove_all_modifiers(db);

            db[self].reset(false);
            db.stack.remove(self);
            let view = db.owner_view_mut(db[self].owner);
            view.exile.shift_remove(&self);
            view.hand.shift_remove(&self);
            view.battlefield.shift_remove(&self);
            view.graveyard.shift_remove(&self);

            for sa in db[self]
                .modified_static_abilities
                .clone()
                .into_iter()
                .collect_vec()
            {
                if let Some(modifier) = db[sa].owned_modifier.take() {
                    modifier.deactivate(db);
                }
            }

            self.apply_modifiers_layered(db);
            true
        }
    }

    pub(crate) fn move_to_exile(
        self,
        db: &mut Database,
        source: CardId,
        reason: Option<ExileReason>,
        duration: Duration,
    ) {
        if self.is_in_location(db, Location::ON_BATTLEFIELD) {
            Log::left_battlefield(db, LeaveReason::Exiled, self);
        }

        if db[self].token {
            self.move_to_limbo(db);
        } else {
            self.remove_all_modifiers(db);

            db[source].exiling.insert(self);

            db[self].reset(matches!(reason, Some(ExileReason::Craft)));

            db[self].exile_reason = reason;
            db[self].exile_duration = Some(duration);

            db.stack.remove(self);
            let view = db.owner_view_mut(db[self].owner);
            view.hand.shift_remove(&self);
            view.library.remove(self);
            view.battlefield.shift_remove(&self);
            view.graveyard.shift_remove(&self);

            view.exile.insert(self);

            for sa in db[self]
                .modified_static_abilities
                .clone()
                .into_iter()
                .collect_vec()
            {
                if let Some(modifier) = db[sa].owned_modifier.take() {
                    modifier.deactivate(db);
                }
            }

            self.apply_modifiers_layered(db);
        }
    }

    pub(crate) fn move_to_limbo(self, db: &mut Database) {
        self.remove_all_modifiers(db);

        db[self].reset(false);
        db.stack.remove(self);
        let view = db.owner_view_mut(db[self].owner);
        view.hand.shift_remove(&self);
        view.library.remove(self);
        view.battlefield.shift_remove(&self);
        view.graveyard.shift_remove(&self);
        view.exile.shift_remove(&self);

        for sa in db[self]
            .modified_static_abilities
            .clone()
            .into_iter()
            .collect_vec()
        {
            if let Some(modifier) = db[sa].owned_modifier.take() {
                modifier.deactivate(db);
            }
        }

        self.apply_modifiers_layered(db);
    }

    pub(crate) fn cleanup_tokens_in_limbo(db: &mut Database) {
        db.cards
            .retain(|id, card| !card.token || db.battlefield[card.controller].contains(id));
    }

    pub(crate) fn remove_all_modifiers(self, db: &mut Database) {
        for modifier in db.modifiers.values_mut() {
            modifier.modifying.remove(&self);
        }
    }

    pub(crate) fn apply_modifiers_layered(self, db: &mut Database) {
        let on_battlefield = self.is_in_location(db, Location::ON_BATTLEFIELD);

        let modifiers = db
            .modifiers
            .iter()
            .filter_map(|(id, modifier)| {
                if modifier.active
                    && (modifier.modifier.modifier.global
                        || (on_battlefield && modifier.modifier.modifier.entire_battlefield)
                        || modifier.modifying.contains(&self))
                {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect_vec();

        let facedown = db[self].facedown && !db[self].transformed;
        let source = if let Some(cloning) = db[self].cloning.as_ref() {
            cloning
        } else {
            self.faceup_face(db)
        };

        let name = if facedown {
            String::default()
        } else {
            source.name.clone()
        };

        let cost = if facedown {
            CastingCost::default()
        } else {
            source.cost.get_or_default().clone()
        };

        let mut base_power = if facedown {
            Some(BasePowerType::Static(2))
        } else if let Some(dynamic) = source.dynamic_power_toughness.as_ref() {
            Some(BasePowerType::Dynamic(dynamic.clone()))
        } else {
            source.power.map(BasePowerType::Static)
        };

        let mut base_toughness = if facedown {
            Some(BaseToughnessType::Static(2))
        } else if let Some(dynamic) = source.dynamic_power_toughness.as_ref() {
            Some(BaseToughnessType::Dynamic(dynamic.clone()))
        } else {
            source.toughness.map(BaseToughnessType::Static)
        };

        let mut types = if facedown {
            TypeSet::from([Type::CREATURE])
        } else {
            TypeSet::from(&source.typeline.types)
        };

        let mut subtypes = if facedown {
            SubtypeSet::default()
        } else {
            SubtypeSet::from(&source.typeline.subtypes)
        };

        let mut keywords = if facedown {
            HashMap::default()
        } else {
            source.keywords.clone()
        };

        let mut colors: HashSet<Color> = if facedown {
            HashSet::default()
        } else {
            source
                .colors
                .iter()
                .map(|c| c.enum_value().unwrap())
                .chain(source.cost.colors())
                .collect()
        };

        let mut triggers: HashMap<TriggerSource, Vec<TriggeredAbility>> = if facedown {
            Default::default()
        } else {
            let mut triggers: HashMap<TriggerSource, Vec<TriggeredAbility>> = Default::default();
            for ability in source.triggered_abilities.iter() {
                triggers
                    .entry(ability.trigger.source.enum_value().unwrap())
                    .or_default()
                    .push(ability.clone());
            }
            triggers
        };

        let mut etb_abilities = if facedown {
            vec![]
        } else {
            source.etb_abilities.clone()
        };

        let mut static_abilities = if facedown {
            HashSet::default()
        } else {
            db[self].static_abilities.clone()
        };

        let mut activated_abilities = if facedown {
            IndexSet::default()
        } else {
            db[self].activated_abilities.clone()
        };

        let mut mana_abilities = if facedown {
            IndexSet::default()
        } else {
            db[self].mana_abilities.clone()
        };

        let mut replacement_abilities = if facedown {
            Default::default()
        } else {
            let mut abilities: HashMap<Replacing, Vec<ReplacementEffect>> = Default::default();
            for ability in source.replacement_abilities.iter() {
                abilities
                    .entry(ability.replacing.enum_value().unwrap())
                    .or_default()
                    .push(ability.clone());
            }
            abilities
        };

        let mut applied_modifiers: HashSet<ModifierId> = Default::default();

        // TODO control changing effects go here
        // TODO text changing effects go here

        for id in modifiers.iter().copied() {
            let modifier = &db[id];
            if !applied_modifiers.contains(&id) {
                let power = base_power.as_ref().map(|base| match base {
                    BasePowerType::Static(value) => *value,
                    BasePowerType::Dynamic(dynamic) => self.dynamic_power_toughness_given_types(
                        db,
                        dynamic,
                        modifier.source,
                        db[self].controller,
                        &types,
                        &subtypes,
                        &keywords,
                        &colors,
                        &activated_abilities,
                    ) as i32,
                });
                let toughness = base_toughness.as_ref().map(|base| match base {
                    BaseToughnessType::Static(value) => *value,
                    BaseToughnessType::Dynamic(dynamic) => {
                        self.dynamic_power_toughness_given_types(
                            db,
                            dynamic,
                            modifier.source,
                            db[self].controller,
                            &types,
                            &subtypes,
                            &keywords,
                            &colors,
                            &activated_abilities,
                        ) as i32
                    }
                });
                if !self.passes_restrictions_given_attributes(
                    db,
                    LogId::current(db),
                    modifier.source,
                    db[self].controller,
                    &modifier.modifier.restrictions,
                    &types,
                    &subtypes,
                    &keywords,
                    &colors,
                    &activated_abilities,
                    power,
                    toughness,
                ) {
                    continue;
                }
            }

            if !modifier.modifier.modifier.add_types.is_empty() {
                applied_modifiers.insert(id);
                types.extend(
                    modifier
                        .modifier
                        .modifier
                        .add_types
                        .keys()
                        .map(|ty| Type::from_i32(*ty).unwrap()),
                );
            }

            if !modifier.modifier.modifier.add_subtypes.is_empty() {
                applied_modifiers.insert(id);
                subtypes.extend(
                    modifier
                        .modifier
                        .modifier
                        .add_subtypes
                        .keys()
                        .map(|ty| Subtype::from_i32(*ty).unwrap()),
                );
            }

            if !modifier.modifier.modifier.remove_types.is_empty() {
                applied_modifiers.insert(id);
                types.retain(|ty| {
                    !modifier
                        .modifier
                        .modifier
                        .remove_types
                        .contains_key(&ty.value())
                });
            }

            if modifier.modifier.modifier.remove_all_types {
                applied_modifiers.insert(id);
                types.clear();
            }

            if !modifier.modifier.modifier.remove_subtypes.is_empty() {
                applied_modifiers.insert(id);
                subtypes.retain(|ty| {
                    !modifier
                        .modifier
                        .modifier
                        .remove_subtypes
                        .contains_key(&ty.value())
                });
            }

            if modifier.modifier.modifier.remove_all_creature_types {
                applied_modifiers.insert(id);
                subtypes.retain(|ty| !ty.is_creature_type());
            }
        }

        for id in modifiers.iter().copied() {
            let modifier = &db[id];
            if !applied_modifiers.contains(&id) {
                let power = base_power.as_ref().map(|base| match base {
                    BasePowerType::Static(value) => *value,
                    BasePowerType::Dynamic(dynamic) => self.dynamic_power_toughness_given_types(
                        db,
                        dynamic,
                        modifier.source,
                        db[self].controller,
                        &types,
                        &subtypes,
                        &keywords,
                        &colors,
                        &activated_abilities,
                    ) as i32,
                });
                let toughness = base_toughness.as_ref().map(|base| match base {
                    BaseToughnessType::Static(value) => *value,
                    BaseToughnessType::Dynamic(dynamic) => {
                        self.dynamic_power_toughness_given_types(
                            db,
                            dynamic,
                            modifier.source,
                            db[self].controller,
                            &types,
                            &subtypes,
                            &keywords,
                            &colors,
                            &activated_abilities,
                        ) as i32
                    }
                });
                if !self.passes_restrictions_given_attributes(
                    db,
                    LogId::current(db),
                    modifier.source,
                    db[self].controller,
                    &modifier.modifier.restrictions,
                    &types,
                    &subtypes,
                    &keywords,
                    &colors,
                    &activated_abilities,
                    power,
                    toughness,
                ) {
                    continue;
                }
            }

            if !modifier.modifier.modifier.add_colors.is_empty() {
                applied_modifiers.insert(id);
                colors.extend(
                    modifier
                        .modifier
                        .modifier
                        .add_colors
                        .iter()
                        .map(|c| c.enum_value().unwrap()),
                );
            }

            if modifier.modifier.modifier.remove_all_colors {
                applied_modifiers.insert(id);
                colors.clear();
            }
        }

        if colors.len() != 1 {
            colors.remove(&Color::COLORLESS);
        }

        let add_keywords = static_abilities
            .iter()
            .filter_map(|sa| {
                if let static_ability::Ability::AddKeywordsIf(AddKeywordsIf {
                    keywords: add_keywords,
                    restrictions,
                    ..
                }) = &db[*sa].ability
                {
                    let power = base_power.as_ref().map(|base| match base {
                        BasePowerType::Static(value) => *value,
                        BasePowerType::Dynamic(dynamic) => {
                            self.dynamic_power_toughness_given_types(
                                db,
                                dynamic,
                                self,
                                db[self].controller,
                                &types,
                                &subtypes,
                                &keywords,
                                &colors,
                                &activated_abilities,
                            ) as i32
                        }
                    });
                    let toughness = base_toughness.as_ref().map(|base| match base {
                        BaseToughnessType::Static(value) => *value,
                        BaseToughnessType::Dynamic(dynamic) => self
                            .dynamic_power_toughness_given_types(
                                db,
                                dynamic,
                                self,
                                db[self].controller,
                                &types,
                                &subtypes,
                                &keywords,
                                &colors,
                                &activated_abilities,
                            ) as i32,
                    });
                    if self.passes_restrictions_given_attributes(
                        db,
                        LogId::current(db),
                        self,
                        db[self].controller,
                        restrictions,
                        &types,
                        &subtypes,
                        &keywords,
                        &colors,
                        &activated_abilities,
                        power,
                        toughness,
                    ) {
                        Some(add_keywords)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect_vec();

        for add in add_keywords {
            for (kw, count) in add.iter() {
                *keywords.entry(*kw).or_default() += count;
            }
        }

        let add_abilities = static_abilities
            .iter()
            .filter_map(|sa| {
                if let static_ability::Ability::AllAbilitiesOfExiledWith(
                    AllAbilitiesOfExiledWith {
                        activation_restrictions,
                        ..
                    },
                ) = &db[*sa].ability
                {
                    let mut add = vec![];
                    for card in db[self].exiling.iter().copied() {
                        add.extend(db[card].activated_abilities.iter().copied());
                    }

                    Some((activation_restrictions.clone(), add))
                } else {
                    None
                }
            })
            .collect_vec();

        for (restrictions, to_add) in add_abilities {
            activated_abilities.extend(to_add.into_iter().map(|id| {
                let mut ability = db[id].ability.clone();
                ability
                    .cost
                    .mut_or_insert_default()
                    .restrictions
                    .extend(restrictions.clone());
                ActivatedAbilityId::upload(db, self, ability)
            }));
        }

        for id in modifiers.iter().copied() {
            let modifier = &db[id];
            if !applied_modifiers.contains(&id) {
                let power = base_power.as_ref().map(|base| match base {
                    BasePowerType::Static(value) => *value,
                    BasePowerType::Dynamic(dynamic) => self.dynamic_power_toughness_given_types(
                        db,
                        dynamic,
                        modifier.source,
                        db[self].controller,
                        &types,
                        &subtypes,
                        &keywords,
                        &colors,
                        &activated_abilities,
                    ) as i32,
                });
                let toughness = base_toughness.as_ref().map(|base| match base {
                    BaseToughnessType::Static(value) => *value,
                    BaseToughnessType::Dynamic(dynamic) => {
                        self.dynamic_power_toughness_given_types(
                            db,
                            dynamic,
                            modifier.source,
                            db[self].controller,
                            &types,
                            &subtypes,
                            &keywords,
                            &colors,
                            &activated_abilities,
                        ) as i32
                    }
                });
                if !self.passes_restrictions_given_attributes(
                    db,
                    LogId::current(db),
                    modifier.source,
                    db[self].controller,
                    &modifier.modifier.restrictions,
                    &types,
                    &subtypes,
                    &keywords,
                    &colors,
                    &activated_abilities,
                    power,
                    toughness,
                ) {
                    continue;
                }
            }

            if modifier.modifier.modifier.remove_all_abilities {
                applied_modifiers.insert(id);

                triggers.clear();
                etb_abilities.clear();
                static_abilities.clear();
                activated_abilities.clear();
                mana_abilities.clear();
                replacement_abilities.clear();
            }

            if !modifier.add_mana_abilities.is_empty() {
                applied_modifiers.insert(id);

                mana_abilities.extend(modifier.add_mana_abilities.iter().copied());
            }

            if !modifier.modifier.modifier.add_static_abilities.is_empty() {
                applied_modifiers.insert(id);

                static_abilities.extend(modifier.add_static_abilities.iter().copied());
            }

            if !modifier.add_activated_abilities.is_empty() {
                applied_modifiers.insert(id);

                activated_abilities.extend(modifier.add_activated_abilities.iter().copied())
            }

            if !modifier.modifier.modifier.remove_keywords.is_empty() {
                applied_modifiers.insert(id);

                keywords
                    .retain(|kw, _| !modifier.modifier.modifier.remove_keywords.contains_key(kw));
            }

            if !modifier.modifier.modifier.add_keywords.is_empty() {
                applied_modifiers.insert(id);

                keywords.extend(modifier.modifier.modifier.add_keywords.clone());
            }
        }

        let mut add_power = 0;
        let mut add_toughness = 0;

        for id in modifiers.iter().copied() {
            let modifier = &db[id];
            if !applied_modifiers.contains(&id) {
                let power = base_power.as_ref().map(|base| match base {
                    BasePowerType::Static(value) => *value,
                    BasePowerType::Dynamic(dynamic) => self.dynamic_power_toughness_given_types(
                        db,
                        dynamic,
                        modifier.source,
                        db[self].controller,
                        &types,
                        &subtypes,
                        &keywords,
                        &colors,
                        &activated_abilities,
                    ) as i32,
                });
                let toughness = base_toughness.as_ref().map(|base| match base {
                    BaseToughnessType::Static(value) => *value,
                    BaseToughnessType::Dynamic(dynamic) => {
                        self.dynamic_power_toughness_given_types(
                            db,
                            dynamic,
                            modifier.source,
                            db[self].controller,
                            &types,
                            &subtypes,
                            &keywords,
                            &colors,
                            &activated_abilities,
                        ) as i32
                    }
                });
                if !self.passes_restrictions_given_attributes(
                    db,
                    LogId::current(db),
                    modifier.source,
                    db[self].controller,
                    &modifier.modifier.restrictions,
                    &types,
                    &subtypes,
                    &keywords,
                    &colors,
                    &activated_abilities,
                    power,
                    toughness,
                ) {
                    continue;
                }
            }

            if let Some(base) = modifier.modifier.modifier.base_power {
                applied_modifiers.insert(id);

                base_power = Some(BasePowerType::Static(base));
            }

            if let Some(base) = modifier.modifier.modifier.base_toughness {
                applied_modifiers.insert(id);

                base_toughness = Some(BaseToughnessType::Static(base));
            }

            if let Some(add) = modifier.modifier.modifier.add_power {
                applied_modifiers.insert(id);

                add_power += add;
            }

            if let Some(add) = modifier.modifier.modifier.add_toughness {
                applied_modifiers.insert(id);

                add_toughness += add;
            }

            if let Some(dynamic) = modifier
                .modifier
                .modifier
                .add_dynamic_power_toughness
                .as_ref()
            {
                let to_add = self.dynamic_power_toughness_given_types(
                    db,
                    dynamic,
                    modifier.source,
                    db[self].controller,
                    &types,
                    &subtypes,
                    &keywords,
                    &colors,
                    &activated_abilities,
                ) as i32;

                add_power += to_add;
                add_toughness += to_add;
            }
        }

        let p1p1 = *db[self].counters.entry(Counter::P1P1).or_default();
        add_power += p1p1 as i32;
        add_toughness += p1p1 as i32;

        let m1m1 = *db[self].counters.entry(Counter::M1M1).or_default();
        add_power -= m1m1 as i32;
        add_toughness -= m1m1 as i32;

        db[self].modified_base_power = base_power;
        db[self].modified_base_toughness = base_toughness;

        db[self].add_power = add_power;
        db[self].modified_cost = cost;
        db[self].modified_name = name;
        db[self].add_toughness = add_toughness;
        db[self].modified_types = types;
        db[self].modified_colors = colors;
        db[self].modified_subtypes = subtypes;
        db[self].modified_triggers = triggers;
        db[self].modified_keywords = keywords;
        db[self].modified_etb_abilities = etb_abilities;
        db[self].modified_mana_abilities = mana_abilities;
        db[self].modified_activated_abilities = activated_abilities;
        db[self].modified_replacement_abilities = replacement_abilities;

        db[self].modified_static_abilities = static_abilities
            .into_iter()
            .inspect(|sa| {
                if let static_ability::Ability::BattlefieldModifier(modifier) = &db[*sa].ability {
                    if db[*sa].owned_modifier.is_none() {
                        let modifier = ModifierId::upload_temporary_modifier(
                            db,
                            db[*sa].source,
                            modifier.clone(),
                        );
                        db[*sa].owned_modifier = Some(modifier);
                        modifier.activate(&mut db.modifiers);
                    }
                }
            })
            .collect();

        let to_deactivate = db
            .static_abilities
            .iter_mut()
            .filter_map(|(id, ability)| {
                if ability.source == self {
                    if !db
                        .cards
                        .get(&self)
                        .unwrap()
                        .modified_static_abilities
                        .contains(id)
                    {
                        ability.owned_modifier.take()
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect_vec();

        for modifier in to_deactivate {
            modifier.deactivate(db);
        }

        db.static_abilities.retain(|id, ability| {
            if ability.source == self {
                if !db
                    .cards
                    .get(&self)
                    .unwrap()
                    .modified_static_abilities
                    .contains(id)
                {
                    db.cards.get(&self).unwrap().static_abilities.contains(id)
                } else {
                    true
                }
            } else {
                true
            }
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn dynamic_power_toughness_given_types(
        self,
        db: &Database,
        dynamic: &DynamicPowerToughness,
        source: CardId,
        self_controller: Controller,
        self_types: &TypeSet,
        self_subtypes: &SubtypeSet,
        self_keywords: &HashMap<i32, u32>,
        self_colors: &HashSet<Color>,
        self_activated_abilities: &IndexSet<ActivatedAbilityId>,
    ) -> usize {
        match dynamic.source.as_ref().unwrap() {
            dynamic_power_toughness::Source::NumberOfCountersOnThis(counter) => {
                if let Counter::ANY = counter.counter.enum_value().unwrap() {
                    db[source].counters.values().sum::<usize>()
                } else {
                    db[source]
                        .counters
                        .get(&counter.counter.enum_value().unwrap())
                        .copied()
                        .unwrap_or_default()
                }
            }
            dynamic_power_toughness::Source::NumberOfPermanentsMatching(matching) => db
                .battlefield
                .battlefields
                .values()
                .flat_map(|battlefield| battlefield.iter())
                .filter(|card| {
                    card.passes_restrictions_given_attributes(
                        db,
                        LogId::current(db),
                        source,
                        self_controller,
                        &matching.restrictions,
                        self_types,
                        self_subtypes,
                        self_keywords,
                        self_colors,
                        self_activated_abilities,
                        None,
                        None,
                    )
                })
                .count(),
        }
    }

    pub(crate) fn apply_modifier(self, db: &mut Database, modifier: ModifierId) {
        db.modifiers
            .get_mut(&modifier)
            .unwrap()
            .modifying
            .insert(self);
        modifier.activate(&mut db.modifiers);
        self.apply_modifiers_layered(db);
    }

    pub(crate) fn has_modes(self, db: &mut Database) -> bool {
        !self.faceup_face(db).modes.is_empty()
    }

    #[allow(unused)]
    pub(crate) fn needs_targets(self, db: &mut Database) -> Vec<usize> {
        let effects = &self.faceup_face(db).effects;
        let aura_targets = self.faceup_face(db).enchant.as_ref().map(|_| 1);
        std::iter::once(())
            .filter_map(|()| aura_targets)
            .chain(
                effects
                    .iter()
                    .map(|effect| effect.effect.as_ref().unwrap().needs_targets(db, self)),
            )
            .collect_vec()
    }

    pub(crate) fn wants_targets(self, db: &mut Database) -> Vec<usize> {
        let effects = self.faceup_face(db).effects.clone();
        let aura_targets = self.faceup_face(db).enchant.as_ref().map(|_| 1);
        std::iter::once(())
            .filter_map(|()| aura_targets)
            .chain(
                effects
                    .into_iter()
                    .map(|effect| effect.effect.as_ref().unwrap().wants_targets(db, self)),
            )
            .collect_vec()
    }

    pub(crate) fn passes_restrictions(
        self,
        db: &Database,
        log_session: LogId,
        source: CardId,
        restrictions: &[Restriction],
    ) -> bool {
        let power = self.power(db);
        let toughness = self.toughness(db);
        self.passes_restrictions_given_attributes(
            db,
            log_session,
            source,
            db[self].controller,
            restrictions,
            &db[self].modified_types,
            &db[self].modified_subtypes,
            &db[self].modified_keywords,
            &db[self].modified_colors,
            &db[self].modified_activated_abilities,
            power,
            toughness,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn passes_restrictions_given_attributes(
        self,
        db: &Database,
        log_session: LogId,
        source: CardId,
        self_controller: Controller,
        restrictions: &[Restriction],
        self_types: &TypeSet,
        self_subtypes: &SubtypeSet,
        self_keywords: &HashMap<i32, u32>,
        self_colors: &HashSet<Color>,
        self_activated_abilities: &IndexSet<ActivatedAbilityId>,
        self_power: Option<i32>,
        self_toughness: Option<i32>,
    ) -> bool {
        for restriction in restrictions.iter() {
            match restriction.restriction.as_ref().unwrap() {
                restriction::Restriction::AttackedThisTurn(_) => {
                    if db.turn.number_of_attackers_this_turn < 1 {
                        return false;
                    }
                }
                restriction::Restriction::Attacking(_) => {
                    if db[self].attacking.is_none() {
                        return false;
                    }
                }
                restriction::Restriction::AttackingOrBlocking(_) => {
                    /*TODO blocking */
                    if db[self].attacking.is_none() {
                        return false;
                    }
                }
                restriction::Restriction::CastFromHand(_) => {
                    if !matches!(db[self].cast_from, Some(CastFrom::Hand)) {
                        return false;
                    }
                }
                restriction::Restriction::Cmc(cmc_test) => {
                    let cmc = db[self].modified_cost.cmc() as i32;
                    match cmc_test.cmc.as_ref().unwrap() {
                        Cmc::Comparison(comparison) => {
                            let matches = match comparison.value.as_ref().unwrap() {
                                comparison::Value::LessThan(i) => cmc < i.value,
                                comparison::Value::LessThanOrEqual(i) => cmc <= i.value,
                                comparison::Value::GreaterThan(i) => cmc > i.value,
                                comparison::Value::GreaterThanOrEqual(i) => cmc >= i.value,
                            };
                            if !matches {
                                return false;
                            }
                        }
                        Cmc::Dynamic(dy) => match dy.dynamic.as_ref().unwrap() {
                            Dynamic::X(_) => {
                                if source.get_x(db) as i32 != cmc {
                                    return false;
                                }
                            }
                        },
                    }
                }
                restriction::Restriction::Controller(controller_restriction) => {
                    match controller_restriction.controller.as_ref().unwrap() {
                        restriction::controller::Controller::Self_(_) => {
                            if db[source].controller != self_controller {
                                return false;
                            }
                        }
                        restriction::controller::Controller::Opponent(_) => {
                            if db[source].controller == self_controller {
                                return false;
                            }
                        }
                    };
                }
                restriction::Restriction::ControllerControlsBlackOrGreen(_) => {
                    let colors = Battlefields::controlled_colors(db, self_controller);
                    if !(colors.contains(&Color::GREEN) || colors.contains(&Color::BLACK)) {
                        return false;
                    }
                }
                restriction::Restriction::ControllerHandEmpty(_) => {
                    if self_controller.has_cards(db, Location::IN_HAND) {
                        return false;
                    }
                }
                restriction::Restriction::ControllerJustCast(_) => {
                    if !Log::session(db, log_session).iter().any(|(_, entry)| {
                        let LogEntry::Cast { card } = entry else {
                            return false;
                        };
                        db[*card].controller == self_controller
                    }) {
                        return false;
                    }
                }
                restriction::Restriction::Descend(count) => {
                    let cards = db.graveyard[self_controller]
                        .iter()
                        .filter(|card| card.is_permanent(db))
                        .count() as i32;
                    if cards < count.count {
                        return false;
                    }
                }
                restriction::Restriction::DescendedThisTurn(_) => {
                    let descended = db
                        .graveyard
                        .descended_this_turn
                        .get(&Owner::from(self_controller))
                        .copied()
                        .unwrap_or_default();
                    if descended < 1 {
                        return false;
                    }
                }
                restriction::Restriction::DuringControllersTurn(_) => {
                    if self_controller != db.turn.active_player() {
                        return false;
                    }
                }
                restriction::Restriction::EnteredBattlefieldThisTurn(
                    EnteredBattlefieldThisTurn {
                        count,
                        restrictions,
                        ..
                    },
                ) => {
                    let entered_this_turn = CardId::entered_battlefield_this_turn(db)
                        .filter(|card| {
                            card.passes_restrictions(db, log_session, source, restrictions)
                        })
                        .count() as i32;
                    if entered_this_turn < *count {
                        return false;
                    }
                }
                restriction::Restriction::HasActivatedAbility(_) => {
                    if self_activated_abilities.is_empty() {
                        return false;
                    }
                }
                restriction::Restriction::IsPermanent(_) => {
                    if !self.is_permanent(db) {
                        return false;
                    }
                }
                restriction::Restriction::InGraveyard(_) => {
                    if !self.is_in_location(db, Location::IN_GRAVEYARD) {
                        return false;
                    }
                }
                restriction::Restriction::Location(restriction::Locations {
                    locations, ..
                }) => {
                    if !locations
                        .iter()
                        .any(|loc| self.is_in_location(db, loc.enum_value().unwrap()))
                    {
                        return false;
                    }
                }
                restriction::Restriction::SpellOrAbilityJustCast(_) => {
                    if !Log::session(db, log_session).iter().any(|(_, entry)| {
                        if let LogEntry::Cast { card } = entry {
                            *card == self
                        } else {
                            false
                        }
                    }) {
                        return false;
                    }
                }
                restriction::Restriction::LifeGainedThisTurn(count) => {
                    let gained_this_turn = db.all_players[self_controller].life_gained_this_turn;
                    if gained_this_turn < count.count {
                        return false;
                    }
                }
                restriction::Restriction::ManaSpentFromSource(source) => {
                    if !db[self]
                        .sourced_mana
                        .contains_key(&source.source.enum_value().unwrap())
                    {
                        return false;
                    }
                }
                restriction::Restriction::NonToken(_) => {
                    if db[self].token {
                        return false;
                    };
                }
                restriction::Restriction::NotChosen(_) => {
                    if Log::session(db, log_session).iter().any(|(_, entry)| {
                        let LogEntry::CardChosen { card } = entry else {
                            return false;
                        };
                        *card == self
                    }) {
                        return false;
                    }
                }
                restriction::Restriction::NotKeywords(not_keywords) => {
                    if self_keywords
                        .keys()
                        .any(|keyword| not_keywords.keywords.contains_key(keyword))
                    {
                        return false;
                    }
                }
                restriction::Restriction::NotOfType(NotOfType {
                    types, subtypes, ..
                }) => {
                    if !types.is_empty()
                        && self_types.iter().any(|ty| types.contains_key(&ty.value()))
                    {
                        return false;
                    }
                    if !subtypes.is_empty()
                        && self_subtypes
                            .iter()
                            .any(|subtype| subtypes.contains_key(&subtype.value()))
                    {
                        return false;
                    }
                }
                restriction::Restriction::NotSelf(_) => {
                    if source == self {
                        return false;
                    }
                }
                restriction::Restriction::NumberOfCountersOnThis(NumberOfCountersOnThis {
                    counter,
                    comparison,
                    ..
                }) => {
                    let count = if let Counter::ANY = counter.enum_value().unwrap() {
                        db[self].counters.values().sum::<usize>()
                    } else {
                        db[self]
                            .counters
                            .get(&counter.enum_value().unwrap())
                            .copied()
                            .unwrap_or_default()
                    } as i32;

                    let matched = match comparison.value.as_ref().unwrap() {
                        comparison::Value::LessThan(value) => count < value.value,
                        comparison::Value::LessThanOrEqual(value) => count <= value.value,
                        comparison::Value::GreaterThan(value) => count > value.value,
                        comparison::Value::GreaterThanOrEqual(value) => count >= value.value,
                    };
                    if !matched {
                        return false;
                    }
                }
                restriction::Restriction::OfColor(OfColor { colors, .. }) => {
                    if !colors
                        .iter()
                        .any(|c| self_colors.contains(&c.enum_value().unwrap()))
                    {
                        return false;
                    }
                }
                restriction::Restriction::OfType(OfType {
                    types, subtypes, ..
                }) => {
                    if !types.is_empty()
                        && !self_types.iter().any(|ty| types.contains_key(&ty.value()))
                    {
                        return false;
                    }
                    if !subtypes.is_empty()
                        && !self_subtypes
                            .iter()
                            .any(|ty| subtypes.contains_key(&ty.value()))
                    {
                        return false;
                    }
                }
                restriction::Restriction::OnBattlefield(_) => {
                    if !self.is_in_location(db, Location::ON_BATTLEFIELD) {
                        return false;
                    }
                }
                restriction::Restriction::Power(comparison) => {
                    if self_power.is_none() {
                        return false;
                    }
                    let power = self_power.unwrap();
                    if !match comparison.comparison.value.as_ref().unwrap() {
                        comparison::Value::LessThan(target) => power < target.value,
                        comparison::Value::LessThanOrEqual(target) => power <= target.value,
                        comparison::Value::GreaterThan(target) => power > target.value,
                        comparison::Value::GreaterThanOrEqual(target) => power >= target.value,
                    } {
                        return false;
                    }
                }
                restriction::Restriction::Self_(_) => {
                    if source != self {
                        return false;
                    }
                }
                restriction::Restriction::SourceCast(_) => {
                    if db[source].cast_from.is_none() {
                        return false;
                    }
                }
                restriction::Restriction::Tapped(_) => {
                    if !self.tapped(db) {
                        return false;
                    }
                }
                restriction::Restriction::TargetedBy(_) => {
                    if !Log::session(db, log_session).iter().any(|(_, entry)| {
                        if let LogEntry::Targeted {
                            source: targeting,
                            target,
                        } = entry
                        {
                            self == *targeting && *target == source
                        } else {
                            false
                        }
                    }) {
                        return false;
                    }
                }
                restriction::Restriction::Threshold(_) => {
                    if db.graveyard[self_controller].len() < 7 {
                        return false;
                    }
                }
                restriction::Restriction::Toughness(comparison) => {
                    if self_toughness.is_none() {
                        return false;
                    }
                    let toughness = self_toughness.unwrap();
                    if !match comparison.comparison.value.as_ref().unwrap() {
                        comparison::Value::LessThan(target) => toughness < target.value,
                        comparison::Value::LessThanOrEqual(target) => toughness <= target.value,
                        comparison::Value::GreaterThan(target) => toughness > target.value,
                        comparison::Value::GreaterThanOrEqual(target) => toughness >= target.value,
                    } {
                        return false;
                    }
                }
            }
        }

        true
    }

    pub(crate) fn apply_aura(self, db: &mut Database, aura_source: CardId) {
        db[aura_source].enchanting = Some(self);

        for modifier in aura_source
            .faceup_face(db)
            .enchant
            .iter()
            .flat_map(|enchant| enchant.modifiers.iter())
            .cloned()
            .collect_vec()
        {
            let modifier = ModifierId::upload_temporary_modifier(db, aura_source, modifier);
            self.apply_modifier(db, modifier);
            db.modifiers
                .get_mut(&modifier)
                .unwrap()
                .modifying
                .insert(self);
            modifier.activate(&mut db.modifiers);
        }
        self.apply_modifiers_layered(db);
    }

    pub(crate) fn marked_damage(self, db: &Database) -> i32 {
        db[self].marked_damage
    }

    pub(crate) fn mark_damage(self, db: &mut Database, amount: u32) {
        db[self].marked_damage += amount as i32;
    }

    pub(crate) fn power(self, db: &Database) -> Option<i32> {
        db[self].modified_base_power.as_ref().map(|power| match power {
                    BasePowerType::Static(value) => *value,
                    BasePowerType::Dynamic(dynamic) => {
                        self.dynamic_power_toughness(db, dynamic) as i32
                    }
                } + db[self].add_power)
    }

    pub(crate) fn toughness(self, db: &Database) -> Option<i32> {
        db[self].modified_base_toughness.as_ref().map(|toughness| match toughness {
                    BaseToughnessType::Static(value) => *value,
                    BaseToughnessType::Dynamic(dynamic) => {
                        self.dynamic_power_toughness(db, dynamic) as i32
                    }
                } + db[self].add_toughness)
    }

    fn dynamic_power_toughness(self, db: &Database, dynamic: &DynamicPowerToughness) -> usize {
        match dynamic.source.as_ref().unwrap() {
            dynamic_power_toughness::Source::NumberOfCountersOnThis(counter) => {
                if let Counter::ANY = counter.counter.enum_value().unwrap() {
                    db[self].counters.values().sum::<usize>()
                } else {
                    db[self]
                        .counters
                        .get(&counter.counter.enum_value().unwrap())
                        .copied()
                        .unwrap_or_default()
                }
            }
            dynamic_power_toughness::Source::NumberOfPermanentsMatching(matching) => db
                .battlefield
                .battlefields
                .values()
                .flat_map(|battlefield| battlefield.iter())
                .filter(|card| {
                    card.passes_restrictions(db, LogId::current(db), self, &matching.restrictions)
                })
                .count(),
        }
    }

    pub(crate) fn types_intersect(self, db: &Database, types: &TypeSet) -> bool {
        types.is_empty()
            || db[self]
                .modified_types
                .iter()
                .any(|type_| types.contains(type_))
    }

    #[allow(unused)]
    pub(crate) fn subtypes_intersect(self, db: &Database, subtypes: &SubtypeSet) -> bool {
        subtypes.is_empty()
            || db[self]
                .modified_subtypes
                .iter()
                .any(|subtype| subtypes.contains(subtype))
    }

    pub(crate) fn targets_for_ability(
        self,
        db: &Database,
        ability: &Ability,
        already_chosen: &HashSet<ActiveTarget>,
    ) -> Vec<Vec<ActiveTarget>> {
        let mut targets = vec![];
        let controller = db[self].controller;
        if !ability.apply_to_self(db) {
            for effect in ability.effects(db).iter() {
                targets.push(effect.effect.as_ref().unwrap().valid_targets(
                    db,
                    self,
                    LogId::current(db),
                    controller,
                    already_chosen,
                ));
            }
        } else {
            targets.push(vec![ActiveTarget::Battlefield { id: self }])
        }

        targets
    }

    pub(crate) fn targets_for_aura(self, db: &Database) -> Option<Vec<ActiveTarget>> {
        if self.faceup_face(db).enchant.is_some() {
            let mut targets = vec![];
            let controller = db[self].controller;
            for card in db.battlefield[controller].iter() {
                if !card.passes_restrictions(
                    db,
                    LogId::current(db),
                    self,
                    &self.faceup_face(db).restrictions,
                ) {
                    continue;
                }

                if !card.can_be_targeted(db, controller) {
                    continue;
                }

                targets.push(ActiveTarget::Battlefield { id: *card });
            }
            Some(targets)
        } else {
            None
        }
    }

    #[instrument(level = Level::DEBUG, skip(db))]
    pub(crate) fn can_be_countered(
        self,
        db: &Database,
        log_session: LogId,
        source: CardId,
        restrictions: &[Restriction],
    ) -> bool {
        if self.faceup_face(db).cannot_be_countered {
            return false;
        }

        if !self.passes_restrictions(db, log_session, source, restrictions) {
            return false;
        }

        for (ability, _) in Battlefields::static_abilities(db) {
            match &ability {
                static_ability::Ability::GreenCannotBeCountered(GreenCannotBeCountered {
                    restrictions,
                    ..
                }) => {
                    if db[self].modified_colors.contains(&Color::GREEN)
                        && self.passes_restrictions(db, log_session, source, restrictions)
                    {
                        return false;
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

        if self.hexproof(db) && db[self].controller != caster {
            return false;
        }

        // TODO protection

        true
    }

    pub(crate) fn can_be_sacrificed(self, _db: &Database) -> bool {
        // TODO
        true
    }

    pub fn tapped(self, db: &Database) -> bool {
        db[self].tapped
    }

    pub(crate) fn tap(self, db: &mut Database) -> PendingResults {
        Log::tapped(db, self);

        db[self].tapped = true;

        let mut pending = PendingResults::default();
        for (listener, trigger) in db.active_triggers_of_source(TriggerSource::TAPPED) {
            if self.passes_restrictions(
                db,
                LogId::current(db),
                listener,
                &trigger.trigger.restrictions,
            ) {
                pending.extend(Stack::move_trigger_to_stack(db, listener, trigger));
            }
        }

        pending
    }

    pub fn untap(self, db: &mut Database) {
        db[self].tapped = false;

        let mut entities = vec![];

        for (id, modifier) in db.modifiers.iter_mut().filter(|(_, modifier)| {
            matches!(
                modifier.modifier.duration.enum_value().unwrap(),
                Duration::UNTIL_UNTAPPED
            )
        }) {
            modifier.modifying.remove(&self);
            if modifier.modifying.is_empty() {
                entities.push(*id);
            }
        }

        for entity in entities {
            entity.deactivate(db);
        }

        self.apply_modifiers_layered(db);
    }

    pub(crate) fn token_copy_of(self, db: &mut Database, controller: Controller) -> CardId {
        let card = clone_card(db, self);

        let id = Self::new();
        db.cards.insert(
            id,
            CardInPlay {
                card,
                controller,
                owner: controller.into(),
                token: true,
                ..Default::default()
            },
        );
        id
    }

    pub(crate) fn clone_card(self, db: &mut Database, cloning: CardId) {
        db[self].cloned_id = Some(cloning);
        db[self].cloning = Some(clone_card(db, cloning));
    }

    pub(crate) fn is_land(self, db: &Database) -> bool {
        self.types_intersect(db, &TypeSet::from([Type::LAND]))
    }

    pub(crate) fn is_permanent(self, db: &Database) -> bool {
        !self.types_intersect(db, &TypeSet::from([Type::INSTANT, Type::SORCERY]))
    }

    pub(crate) fn shroud(self, db: &Database) -> bool {
        db[self]
            .modified_keywords
            .contains_key(&Keyword::SHROUD.value())
    }

    pub(crate) fn hexproof(self, db: &Database) -> bool {
        db[self]
            .modified_keywords
            .contains_key(&Keyword::HEXPROOF.value())
    }

    #[allow(unused)]
    pub(crate) fn flying(self, db: &Database) -> bool {
        db[self]
            .modified_keywords
            .contains_key(&Keyword::FLYING.value())
    }

    pub(crate) fn indestructible(self, db: &Database) -> bool {
        db[self]
            .modified_keywords
            .contains_key(&Keyword::INDESTRUCTIBLE.value())
    }

    pub(crate) fn vigilance(self, db: &Database) -> bool {
        db[self]
            .modified_keywords
            .contains_key(&Keyword::VIGILANCE.value())
    }

    pub fn name(self, db: &Database) -> &String {
        &db[self].modified_name
    }

    pub(crate) fn has_flash(self, db: &Database) -> bool {
        db[self]
            .modified_keywords
            .contains_key(&Keyword::FLASH.value())
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

    pub fn modified_by_text(self, db: &Database) -> Vec<String> {
        self.modified_by(db)
            .into_iter()
            .map(|card| card.name(db))
            .cloned()
            .collect_vec()
    }

    pub fn modified_by(self, db: &Database) -> Vec<CardId> {
        db.modifiers
            .values()
            .filter_map(|modifier| {
                if modifier.modifying.contains(&self) {
                    Some(modifier.source)
                } else {
                    None
                }
            })
            .collect_vec()
    }

    pub(crate) fn cascade(self, db: &mut Database) -> u32 {
        db[self]
            .modified_keywords
            .get(&Keyword::CASCADE.value())
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn exiled_with_cascade(db: &mut Database) -> Vec<CardId> {
        db.exile
            .exile_zones
            .values()
            .flat_map(|e| e.iter())
            .copied()
            .filter(|card| matches!(db[*card].exile_reason, Some(ExileReason::Cascade)))
            .collect_vec()
    }

    pub(crate) fn get_x(self, db: &Database) -> usize {
        db[self].x_is
    }

    pub(crate) fn mana_from_source(self, db: &mut Database, sources: &[ManaSource]) {
        let mut sourced = HashMap::default();
        for source in sources.iter().copied() {
            *sourced.entry(source).or_default() += 1
        }

        db[self].sourced_mana = sourced;
    }

    pub(crate) fn can_attack(self, db: &Database) -> bool {
        self.types_intersect(db, &TypeSet::from([Type::CREATURE]))
            && !db[self].modified_static_abilities.iter().any(|ability| {
                matches!(
                    db[*ability].ability,
                    static_ability::Ability::PreventAttacks(_)
                )
            })
    }

    pub(crate) fn battle_cry(self, db: &Database) -> u32 {
        db[self]
            .modified_keywords
            .get(&Keyword::BATTLE_CRY.value())
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn target_from_location(self, db: &Database) -> ActiveTarget {
        if db.battlefield[db[self].controller].contains(&self) {
            ActiveTarget::Battlefield { id: self }
        } else if db.graveyard[db[self].owner].contains(&self) {
            ActiveTarget::Graveyard { id: self }
        } else if db.all_players[db[self].owner].library.cards.contains(&self) {
            ActiveTarget::Library { id: self }
        } else if db.exile[db[self].owner].contains(&self) {
            ActiveTarget::Exile { id: self }
        } else if db.hand[db[self].owner].contains(&self) {
            ActiveTarget::Hand { id: self }
        } else {
            ActiveTarget::Stack {
                id: db.stack.find(self).unwrap(),
            }
        }
    }
}

impl Default for CardId {
    fn default() -> Self {
        Self::new()
    }
}

fn clone_card(db: &mut Database, cloning: CardId) -> Card {
    Card {
        name: cloning.faceup_face(db).name.clone(),
        typeline: cloning.faceup_face(db).typeline.clone(),
        cost: cloning.faceup_face(db).cost.clone(),
        cost_reducer: Default::default(),
        cannot_be_countered: false,
        colors: cloning.faceup_face(db).colors.clone(),
        oracle_text: String::default(),
        enchant: cloning.faceup_face(db).enchant.clone(),
        effects: cloning.faceup_face(db).effects.clone(),
        modes: cloning.faceup_face(db).modes.clone(),
        etb_abilities: cloning.faceup_face(db).etb_abilities.clone(),
        apply_individually: cloning.faceup_face(db).apply_individually,
        static_abilities: cloning.faceup_face(db).static_abilities.clone(),
        activated_abilities: cloning.faceup_face(db).activated_abilities.clone(),
        triggered_abilities: cloning.faceup_face(db).triggered_abilities.clone(),
        replacement_abilities: cloning.faceup_face(db).replacement_abilities.clone(),
        mana_abilities: cloning.faceup_face(db).mana_abilities.clone(),
        dynamic_power_toughness: cloning.faceup_face(db).dynamic_power_toughness.clone(),
        power: cloning.faceup_face(db).power,
        toughness: cloning.faceup_face(db).toughness,
        etb_tapped: cloning.faceup_face(db).etb_tapped,
        keywords: cloning.faceup_face(db).keywords.clone(),
        restrictions: cloning.faceup_face(db).restrictions.clone(),
        back_face: protobuf::MessageField::none(),
        ..Default::default()
    }
}
