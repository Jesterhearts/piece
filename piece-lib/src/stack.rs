use std::hash::Hash;

use indexmap::IndexMap;
use itertools::Itertools;
use protobuf::Enum;
use uuid::Uuid;

use crate::{
    abilities::Ability,
    effects::{EffectBundle, PendingEffects, SelectedStack, SelectionResult},
    in_play::{CardId, CastFrom, Database},
    log::{Log, LogId},
    player::Owner,
    protogen::{
        effects::{
            pay_cost::PayMana, ClearSelected, CompleteSpellResolution, Effect, MoveToStack,
            PayCost, PayCosts, PushSelected, ReplacementEffect, TriggeredAbility,
        },
        keywords::Keyword,
        mana::{
            spend_reason::{Casting, Reason},
            SpendReason,
        },
        targets::{Location, Restriction},
        triggers::TriggerSource,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StackId(Uuid);

impl StackId {
    pub(crate) fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for StackId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

#[derive(Debug)]
enum ResolutionType {
    Card(CardId),
    Ability(CardId),
}

#[derive(Debug, Clone)]
pub enum TargetType {
    Card(CardId),
    Stack(StackId),
    Ability { source: CardId, ability: Ability },
    ReplacementAbility(ReplacementEffect),
    Player(Owner),
}

#[derive(Debug, Clone)]
pub struct Selected {
    pub(crate) location: Option<Location>,
    pub target_type: TargetType,
    pub(crate) targeted: bool,

    pub(crate) restrictions: Vec<Restriction>,
}

impl Selected {
    pub(crate) fn display(&self, db: &Database) -> String {
        match &self.target_type {
            TargetType::Card(id) => id.name(db).clone(),
            TargetType::Stack(id) => db.stack.entries.get(id).unwrap().display(db),
            TargetType::ReplacementAbility(effect) => effect
                .effects
                .iter()
                .map(|effect| &effect.oracle_text)
                .join(" "),
            TargetType::Player(id) => db.all_players[*id].name.clone(),
            TargetType::Ability { ability, .. } => ability.text(db),
        }
    }

    pub(crate) fn id(&self, db: &Database) -> Option<CardId> {
        match &self.target_type {
            TargetType::Card(card) => Some(*card),
            TargetType::Stack(stack) => db.stack.entries.get(stack).and_then(|entry| {
                if let Entry::Card(card) = entry.ty {
                    Some(card)
                } else {
                    None
                }
            }),
            TargetType::Ability { .. } => None,
            TargetType::ReplacementAbility(_) => None,
            TargetType::Player(_) => None,
        }
    }

    pub(crate) fn player(&self) -> Option<Owner> {
        match &self.target_type {
            TargetType::Player(id) => Some(*id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Entry {
    Card(CardId),
    Ability { source: CardId, ability: Ability },
}

#[derive(Debug, Clone)]
pub struct StackEntry {
    pub(crate) targets: Vec<Selected>,
    pub(crate) ty: Entry,
    pub(crate) modes: Vec<usize>,
    pub(crate) settled: bool,
}

impl StackEntry {
    pub fn display(&self, db: &Database) -> String {
        match &self.ty {
            Entry::Card(card) => card.faceup_face(db).name.clone(),
            Entry::Ability {
                source: card_source,
                ability,
            } => {
                format!("{}: {}", db[*card_source].modified_name, ability.text(db))
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct Stack {
    pub(crate) entries: IndexMap<StackId, StackEntry>,
}

impl Stack {
    pub(crate) fn find(&self, card: CardId) -> Option<StackId> {
        self.entries
            .iter()
            .rev()
            .find(|(_, entry)| match &entry.ty {
                Entry::Card(entry) => *entry == card,
                Entry::Ability { source, .. } => *source == card,
            })
            .map(|(id, _)| *id)
    }

    pub(crate) fn split_second(&self, db: &Database) -> bool {
        if let Some((
            _,
            StackEntry {
                ty: Entry::Card(card),
                ..
            },
        )) = self.entries.last()
        {
            db[*card]
                .modified_keywords
                .contains_key(&Keyword::SPLIT_SECOND.value())
        } else {
            false
        }
    }

    pub(crate) fn remove(&mut self, card: CardId) {
        self.entries
            .retain(|_, entry| !matches!(entry.ty, Entry::Card(entry) if entry == card));
    }

    #[cfg(test)]
    pub(crate) fn target_nth(&self, nth: usize) -> Selected {
        let id = self.entries.get_index(nth).unwrap().0;
        Selected {
            location: Some(Location::IN_STACK),
            target_type: TargetType::Stack(*id),
            targeted: true,
            restrictions: vec![],
        }
    }

    pub fn entries(&self) -> &IndexMap<StackId, StackEntry> {
        &self.entries
    }

    pub fn entries_unsettled(&self) -> Vec<StackEntry> {
        self.entries
            .values()
            .filter(|entry| !entry.settled)
            .cloned()
            .collect_vec()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn settle(&mut self) {
        for entry in self.entries.values_mut() {
            entry.settled = true;
        }
    }

    pub fn resolve_1(db: &mut Database) -> PendingEffects {
        let Some((_, next)) = db.stack.entries.pop() else {
            return PendingEffects::default();
        };

        db.stack.settle();

        let (effects, resolving_card, source, ty) = match next.ty {
            Entry::Card(card) => (
                card.faceup_face(db).effects.clone(),
                Some(card),
                card,
                ResolutionType::Card(card),
            ),
            Entry::Ability { source, ability } => (
                ability.effects(db),
                None,
                source,
                ResolutionType::Ability(source),
            ),
        };

        assert!(next.targets.len() <= 1);
        let mut pending = PendingEffects::new(SelectedStack::new(next.targets.clone()));
        pending.selected.modes = next.modes;
        pending.push_front(EffectBundle {
            effects,
            source: Some(source),
            ..Default::default()
        });

        while !pending.wants_input(db) {
            if let SelectionResult::Complete = pending.resolve(db, None) {
                break;
            }
        }

        if let Some(resolving_card) = resolving_card {
            if resolving_card.is_permanent(db) {
                pending.push_back(EffectBundle {
                    push_on_enter: Some(next.targets),
                    ..Default::default()
                });
                pending.push_back(EffectBundle {
                    push_on_enter: Some(vec![Selected {
                        location: Some(Location::IN_STACK),
                        target_type: TargetType::Card(resolving_card),
                        targeted: false,
                        restrictions: vec![],
                    }]),
                    source: Some(resolving_card),
                    effects: vec![CompleteSpellResolution::default().into()],
                    ..Default::default()
                });
            } else {
                pending.push_back(EffectBundle {
                    push_on_enter: Some(vec![Selected {
                        location: Some(Location::IN_STACK),
                        target_type: TargetType::Card(resolving_card),
                        targeted: false,
                        restrictions: vec![],
                    }]),
                    source: Some(resolving_card),
                    effects: vec![CompleteSpellResolution::default().into()],
                    ..Default::default()
                });
            }

            if let ResolutionType::Ability(_) = ty {
                Log::ability_resolved(db, source);
            }
        }

        pending
    }

    pub(crate) fn move_trigger_to_stack(
        _db: &mut Database,
        listener: CardId,
        trigger: TriggeredAbility,
    ) -> EffectBundle {
        let mut to_trigger = vec![
            Effect::from(PushSelected::default()),
            Effect::from(ClearSelected::default()),
        ];
        if let Some(targets) = trigger.targets.as_ref() {
            to_trigger.push(targets.clone().into());
        }
        if let Some(modes) = trigger.modes.as_ref() {
            to_trigger.push(modes.clone().into());
        }
        to_trigger.push(Effect {
            effect: Some(MoveToStack::default().into()),
            ..Default::default()
        });
        EffectBundle {
            push_on_enter: Some(vec![Selected {
                location: Some(Location::ON_BATTLEFIELD),
                target_type: TargetType::Ability {
                    source: listener,
                    ability: Ability::TriggeredAbility(trigger),
                },
                targeted: false,
                restrictions: vec![],
            }]),
            source: Some(listener),
            effects: to_trigger,
            ..Default::default()
        }
    }

    pub(crate) fn move_card_to_stack_from_hand(db: &mut Database, card: CardId) -> PendingEffects {
        db[card].cast_from = Some(CastFrom::Hand);

        let mut pending = PendingEffects::default();
        pending.push_front(Stack::prepare_card_for_stack(db, card, true));

        pending
    }

    pub(crate) fn push_card(
        db: &mut Database,
        source: CardId,
        targets: Vec<Selected>,
        chosen_modes: Vec<usize>,
    ) -> Vec<EffectBundle> {
        db.stack.entries.insert(
            StackId::new(),
            StackEntry {
                ty: Entry::Card(source),
                targets: targets.clone(),
                settled: true,
                modes: chosen_modes,
            },
        );

        let mut effects = vec![];

        for (listener, trigger) in db.active_triggers_of_source(TriggerSource::CAST) {
            if source.passes_restrictions(
                db,
                LogId::current(db),
                listener,
                &trigger.trigger.restrictions,
            ) {
                effects.push(Stack::move_trigger_to_stack(db, listener, trigger));
            }
        }

        for target in targets.into_iter() {
            if let Some(Location::ON_BATTLEFIELD) = target.location {
                for (listener, trigger) in db.active_triggers_of_source(TriggerSource::TARGETED) {
                    if listener == target.id(db).unwrap()
                        && source.passes_restrictions(
                            db,
                            LogId::current(db),
                            listener,
                            &trigger.trigger.restrictions,
                        )
                    {
                        effects.push(Stack::move_trigger_to_stack(db, listener, trigger));
                    }
                }
            }
        }

        effects
    }

    pub(crate) fn push_ability(
        db: &mut Database,
        source: CardId,
        ability: Ability,
        targets: Vec<Selected>,
    ) -> Vec<EffectBundle> {
        db.stack.entries.insert(
            StackId::new(),
            StackEntry {
                ty: Entry::Ability { source, ability },
                targets: targets.clone(),
                modes: vec![],
                settled: true,
            },
        );

        let mut pending = vec![];
        for target in targets.into_iter() {
            if let Some(Location::ON_BATTLEFIELD) = target.location {
                for (listener, trigger) in db.active_triggers_of_source(TriggerSource::TARGETED) {
                    if listener == target.id(db).unwrap()
                        && source.passes_restrictions(
                            db,
                            LogId::current(db),
                            listener,
                            &trigger.trigger.restrictions,
                        )
                    {
                        pending.push(Stack::move_trigger_to_stack(db, listener, trigger));
                    }
                }
            }
        }

        pending
    }

    pub(crate) fn prepare_card_for_stack(
        db: &mut Database,
        card: CardId,
        pay_costs: bool,
    ) -> EffectBundle {
        let mut to_cast = vec![
            Effect {
                effect: Some(PushSelected::default().into()),
                ..Default::default()
            },
            Effect {
                effect: Some(ClearSelected::default().into()),
                ..Default::default()
            },
        ];
        if let Some(modes) = card.faceup_face(db).modes.as_ref() {
            to_cast.push(modes.clone().into());
        }
        if let Some(target) = card.faceup_face(db).targets.as_ref() {
            to_cast.push(target.clone().into());
        }
        to_cast.push(
            card.faceup_face(db)
                .additional_costs
                .get_or_default()
                .clone()
                .into(),
        );
        if pay_costs {
            to_cast.push(Effect {
                effect: Some(
                    PayCosts {
                        pay_costs: vec![PayCost {
                            cost: Some(
                                PayMana {
                                    paying: db[card]
                                        .modified_cost
                                        .mana_cost
                                        .iter()
                                        .cloned()
                                        .sorted()
                                        .collect_vec(),
                                    reducer: card.faceup_face(db).cost_reducer.clone(),
                                    reason: protobuf::MessageField::some(SpendReason {
                                        reason: Some(Reason::Casting(Casting {
                                            card: protobuf::MessageField::some(card.into()),
                                            ..Default::default()
                                        })),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }
                                .into(),
                            ),
                            ..Default::default()
                        }],
                        ..Default::default()
                    }
                    .into(),
                ),
                ..Default::default()
            });
        }

        to_cast.push(Effect {
            effect: Some(MoveToStack::default().into()),
            ..Default::default()
        });

        EffectBundle {
            push_on_enter: Some(vec![Selected {
                location: Some(Location::IN_HAND),
                target_type: TargetType::Card(card),
                targeted: false,
                restrictions: vec![],
            }]),
            effects: to_cast,
            source: Some(card),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use pretty_assertions::assert_eq;

    use crate::{
        effects::{PendingEffects, SelectionResult},
        in_play::{CardId, CastFrom, Database},
        load_cards,
        player::AllPlayers,
        stack::Stack,
    };

    #[test]
    fn resolves_creatures() -> anyhow::Result<()> {
        let cards = load_cards()?;
        let mut all_players = AllPlayers::default();
        let player = all_players.new_player("Player".to_string(), 20);
        let mut db = Database::new(all_players);
        let card1 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");

        let mut results = PendingEffects::default();
        results.apply_results(card1.move_to_stack(
            &mut db,
            Default::default(),
            CastFrom::Hand,
            vec![],
        ));
        let result = results.resolve(&mut db, None);
        assert_eq!(result, SelectionResult::Complete);

        let mut results = Stack::resolve_1(&mut db);

        let result = results.resolve(&mut db, None);
        assert_eq!(result, SelectionResult::TryAgain);
        let result = results.resolve(&mut db, None);
        assert_eq!(result, SelectionResult::Complete);

        assert!(db.stack.is_empty());
        assert_eq!(
            db.battlefield
                .battlefields
                .values()
                .flat_map(|b| b.iter())
                .copied()
                .collect_vec(),
            [card1]
        );

        Ok(())
    }
}
