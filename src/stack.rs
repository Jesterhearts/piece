use std::collections::HashSet;

use derive_more::{Deref, DerefMut};
use itertools::Itertools;

use crate::{
    abilities::{Ability, TriggeredAbility},
    battlefield::ActionResult,
    card::Keyword,
    cost::AdditionalCost,
    effects::EffectBehaviors,
    in_play::{CardId, CastFrom, Database},
    log::{Log, LogId},
    pending_results::{
        choose_targets::ChooseTargets,
        pay_costs::SpendMana,
        pay_costs::TapPermanent,
        pay_costs::{ExileCards, ExilePermanentsCmcX},
        pay_costs::{ExileCardsSharingType, PayCost},
        pay_costs::{SacrificePermanent, TapPermanentsPowerXOrMore},
        PendingResults, Source, TargetSource,
    },
    player::{mana_pool::SpendReason, Owner},
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct Settled;

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct Targets(pub(crate) Vec<Vec<ActiveTarget>>);

#[derive(Debug)]
enum ResolutionType {
    Card(CardId),
    Ability(CardId),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub(crate) enum ActiveTarget {
    Stack { id: usize },
    Battlefield { id: CardId },
    Graveyard { id: CardId },
    Library { id: CardId },
    Player { id: Owner },
}

impl ActiveTarget {
    pub(crate) fn display(&self, db: &Database) -> String {
        match self {
            ActiveTarget::Stack { id } => {
                format!("Stack ({}): {}", id, db.stack.entries[*id].display(db))
            }
            ActiveTarget::Battlefield { id } => {
                format!("{} - ({})", id.name(db), id)
            }
            ActiveTarget::Graveyard { id } => {
                format!("{} - ({})", id.name(db), id)
            }
            ActiveTarget::Library { id } => {
                format!("{} - ({})", id.name(db), id)
            }
            ActiveTarget::Player { id } => db.all_players[*id].name.clone(),
        }
    }

    pub(crate) fn id(&self) -> Option<CardId> {
        match self {
            ActiveTarget::Battlefield { id }
            | ActiveTarget::Graveyard { id }
            | ActiveTarget::Library { id } => Some(*id),
            ActiveTarget::Stack { .. } => None,
            ActiveTarget::Player { .. } => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Entry {
    Card(CardId),
    Ability { source: CardId, ability: Ability },
}

impl Entry {
    pub(crate) fn source(&self) -> CardId {
        match self {
            Entry::Card(source) | Entry::Ability { source, .. } => *source,
        }
    }
}

#[derive(Debug, Clone, Deref, DerefMut)]
pub(crate) struct Modes(pub(crate) Vec<usize>);

#[derive(Debug, Clone)]
pub struct StackEntry {
    pub(crate) ty: Entry,
    pub(crate) targets: Vec<Vec<ActiveTarget>>,
    pub(crate) mode: Vec<usize>,
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
                format!("{}: {}", db[*card_source].modified_name, ability.text())
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct Stack {
    pub(crate) entries: Vec<StackEntry>,
}

impl Stack {
    pub(crate) fn contains(&self, card: CardId) -> bool {
        self.entries
            .iter()
            .any(|entry| matches!(entry.ty, Entry::Card(entry) if entry == card))
    }

    pub(crate) fn split_second(&self, db: &Database) -> bool {
        if let Some(StackEntry {
            ty: Entry::Card(card),
            ..
        }) = self.entries.last()
        {
            db[*card]
                .modified_keywords
                .contains_key(&Keyword::SplitSecond)
        } else {
            false
        }
    }

    pub(crate) fn remove(&mut self, card: CardId) {
        self.entries
            .retain(|entry| !matches!(entry.ty, Entry::Card(entry) if entry == card));
    }

    #[cfg(test)]
    pub(crate) fn target_nth(&self, nth: usize) -> ActiveTarget {
        ActiveTarget::Stack { id: nth }
    }

    pub fn entries(&self) -> &Vec<StackEntry> {
        &self.entries
    }

    pub fn entries_unsettled(&self) -> Vec<StackEntry> {
        self.entries
            .iter()
            .filter(|entry| !entry.settled)
            .cloned()
            .collect_vec()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn settle(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.settled = true;
        }
    }

    pub fn resolve_1(db: &mut Database) -> PendingResults {
        let Some(next) = db.stack.entries.pop() else {
            return PendingResults::default();
        };

        db.stack.settle();

        let (apply_to_self, effects, controller, resolving_card, source, ty) = match next.ty {
            Entry::Card(card) => {
                let effects = if !card.faceup_face(db).modes.is_empty() {
                    debug!("Modes: {:?}", card.faceup_face(db).modes);
                    card.faceup_face(db).modes[next.mode.into_iter().exactly_one().unwrap()]
                        .effects
                        .clone()
                } else {
                    card.faceup_face(db).effects.clone()
                };

                (
                    false,
                    effects,
                    db[card].controller,
                    Some(card),
                    card,
                    ResolutionType::Card(card),
                )
            }
            Entry::Ability { source, ability } => (
                ability.apply_to_self(),
                ability.effects(),
                db[source].controller,
                None,
                source,
                ResolutionType::Ability(source),
            ),
        };

        let mut results = PendingResults::default();
        results.apply_in_stages();

        let mut targets = next.targets.into_iter();
        for (effect, targets) in effects
            .into_iter()
            .zip((&mut targets).chain(std::iter::repeat(vec![])))
        {
            let effect = effect.effect;
            if targets.len() != effect.needs_targets(db, source)
                && effect.needs_targets(db, source) != 0
            {
                let valid_targets =
                    effect.valid_targets(db, source, controller, &HashSet::default());
                results.push_choose_targets(ChooseTargets::new(
                    TargetSource::Effect(effect),
                    valid_targets,
                    source,
                ));
                continue;
            }

            if effect.wants_targets(db, source) > 0 {
                let valid_targets = effect
                    .valid_targets(db, source, controller, &HashSet::default())
                    .into_iter()
                    .collect::<HashSet<_>>();
                if !targets.iter().all(|target| valid_targets.contains(target)) {
                    warn!(
                        "Did not match targets: {:?} vs valid {:?}",
                        targets, valid_targets
                    );
                    if let Some(resolving_card) = resolving_card {
                        let mut results = PendingResults::default();
                        results.push_settled(ActionResult::StackToGraveyard(resolving_card));
                        return results;
                    } else {
                        return PendingResults::default();
                    }
                }
            }

            effect.push_behavior_with_targets(
                db,
                targets,
                apply_to_self,
                source,
                controller,
                &mut results,
            );
        }

        if let Some(resolving_card) = resolving_card {
            if resolving_card.is_permanent(db) {
                results.push_settled(ActionResult::AddToBattlefield(
                    resolving_card,
                    targets.next().and_then(|targets| {
                        targets.into_iter().find_map(|target| match target {
                            ActiveTarget::Battlefield { id } => Some(id),
                            _ => None,
                        })
                    }),
                ));
            } else {
                results.push_settled(ActionResult::StackToGraveyard(resolving_card));
            }
        }

        let id = LogId::new();
        match ty {
            ResolutionType::Card(card) => Log::spell_resolved(db, id, card),
            ResolutionType::Ability(source) => Log::ability_resolved(db, id, source),
        }

        results
    }

    pub(crate) fn move_etb_ability_to_stack(
        db: &mut Database,
        ability: Ability,
        source: CardId,
    ) -> PendingResults {
        let mut results = PendingResults::default();

        let targets = source.targets_for_ability(db, &ability, &HashSet::default());
        results.push_settled(ActionResult::AddAbilityToStack {
            ability,
            source,
            targets,
            x_is: None,
        });

        results
    }

    pub(crate) fn move_trigger_to_stack(
        db: &mut Database,
        listener: CardId,
        trigger: TriggeredAbility,
    ) -> PendingResults {
        let mut results = PendingResults::default();

        let mut targets = vec![];
        let controller = db[listener].controller;
        for effect in trigger.effects.iter() {
            targets.push(effect.effect.valid_targets(
                db,
                listener,
                controller,
                &HashSet::default(),
            ));
        }

        results.push_settled(ActionResult::AddTriggerToStack {
            source: listener,
            trigger,
            targets,
        });

        results
    }

    pub(crate) fn move_card_to_stack_from_hand(
        db: &mut Database,
        card: CardId,
        paying_costs: bool,
    ) -> PendingResults {
        add_card_to_stack(card, db, CastFrom::Hand, paying_costs)
    }

    pub(crate) fn move_card_to_stack_from_exile(
        db: &mut Database,
        card: CardId,
        paying_costs: bool,
    ) -> PendingResults {
        add_card_to_stack(card, db, CastFrom::Exile, paying_costs)
    }

    pub(crate) fn push_card(
        &mut self,
        arg: CardId,
        targets: Vec<Vec<ActiveTarget>>,
        chosen_modes: Vec<usize>,
    ) {
        self.entries.push(StackEntry {
            ty: Entry::Card(arg),
            targets,
            settled: true,
            mode: chosen_modes,
        });
    }

    pub(crate) fn push_ability(
        &mut self,
        source: CardId,
        ability: Ability,
        targets: Vec<Vec<ActiveTarget>>,
    ) {
        self.entries.push(StackEntry {
            ty: Entry::Ability { source, ability },
            targets,
            mode: vec![],
            settled: false,
        })
    }
}

fn add_card_to_stack(
    card: CardId,
    db: &mut Database,
    from: CastFrom,
    paying_costs: bool,
) -> PendingResults {
    let mut results = PendingResults::default();

    db[card].cast_from = Some(from);
    card.apply_modifiers_layered(db);

    if card.has_modes(db) {
        results.push_choose_mode(Source::Card(card));
    }

    results.add_card_to_stack(card, from);
    if card.wants_targets(db).into_iter().sum::<usize>() > 0 {
        let controller = db[card].controller;
        if card.faceup_face(db).enchant.is_some() {
            results.push_choose_targets(ChooseTargets::new(
                TargetSource::Aura(card),
                card.targets_for_aura(db).unwrap(),
                card,
            ))
        }

        if card.faceup_face(db).effects.len() == 1 {
            let effect = &card
                .faceup_face(db)
                .effects
                .iter()
                .exactly_one()
                .unwrap()
                .effect;
            let valid_targets = effect.valid_targets(db, card, controller, &HashSet::default());
            if valid_targets.len() < effect.needs_targets(db, card) {
                return PendingResults::default();
            }

            results.push_choose_targets(ChooseTargets::new(
                TargetSource::Effect(effect.clone()),
                valid_targets,
                card,
            ));
        } else {
            for effect in card.faceup_face(db).effects.iter() {
                let valid_targets =
                    effect
                        .effect
                        .valid_targets(db, card, controller, &HashSet::default());
                results.push_choose_targets(ChooseTargets::new(
                    TargetSource::Effect(effect.effect.clone()),
                    valid_targets,
                    card,
                ));
            }
        }
    }

    // It is important that paying costs happens last, because some cards have effects that depend on what they are targeting.
    let cost = &card.faceup_face(db).cost;
    if paying_costs {
        results.push_pay_costs(PayCost::SpendMana(SpendMana::new(
            cost.mana_cost.clone(),
            card,
            SpendReason::Casting(card),
        )));
    }
    for cost in cost.additional_cost.iter() {
        match cost {
            AdditionalCost::DiscardThis => unreachable!(),
            AdditionalCost::SacrificeSource => unreachable!(),
            AdditionalCost::RemoveCounter { .. } => unreachable!(),
            AdditionalCost::PayLife(_) => todo!(),
            AdditionalCost::SacrificePermanent(restrictions) => {
                results.push_pay_costs(PayCost::SacrificePermanent(SacrificePermanent::new(
                    restrictions.clone(),
                    card,
                )));
            }
            AdditionalCost::TapPermanent(restrictions) => {
                results.push_pay_costs(PayCost::TapPermanent(TapPermanent::new(
                    restrictions.clone(),
                    card,
                )));
            }
            AdditionalCost::TapPermanentsPowerXOrMore { x_is, restrictions } => {
                results.push_pay_costs(PayCost::TapPermanentsPowerXOrMore(
                    TapPermanentsPowerXOrMore::new(restrictions.clone(), *x_is, card),
                ));
            }
            AdditionalCost::ExileCardsCmcX(restrictions) => {
                results.push_pay_costs(PayCost::ExilePermanentsCmcX(ExilePermanentsCmcX::new(
                    restrictions.clone(),
                    card,
                )));
            }
            AdditionalCost::ExileCard { restrictions } => {
                results.push_pay_costs(PayCost::ExileCards(ExileCards::new(
                    None,
                    1,
                    1,
                    restrictions.clone(),
                    card,
                )));
            }
            AdditionalCost::ExileXOrMoreCards {
                minimum,
                restrictions,
            } => {
                results.push_pay_costs(PayCost::ExileCards(ExileCards::new(
                    None,
                    *minimum,
                    usize::MAX,
                    restrictions.clone(),
                    card,
                )));
            }
            AdditionalCost::ExileSharingCardType { count } => {
                results.push_pay_costs(PayCost::ExileCardsSharingType(ExileCardsSharingType::new(
                    None, card, *count,
                )));
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::{
        in_play::{CardId, Database},
        load_cards,
        pending_results::ResolutionResult,
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

        card1.move_to_stack(&mut db, Default::default(), None, vec![]);

        let mut results = Stack::resolve_1(&mut db);

        let result = results.resolve(&mut db, None);
        assert_eq!(result, ResolutionResult::Complete);

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
