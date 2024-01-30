mod exile_cards_sharing_type;
mod exile_permanents;
mod exile_permanents_cmc_x;
mod pay_cost;
mod pay_life;
mod pay_mana;
mod sacrifice_permanent;
mod tap_permanent;
mod tap_permanents_power_x_or_more;

use crate::{
    effects::{ApplyResult, EffectBehaviors, Options, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    player::Owner,
    protogen::effects::PayCosts,
    stack::Selected,
};

impl EffectBehaviors for PayCosts {
    fn wants_input(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        modes: &[usize],
    ) -> bool {
        self.pay_costs[self.paying as usize].wants_input(db, source, already_selected, modes)
    }

    fn priority(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        _modes: &[usize],
    ) -> Owner {
        if let Some(player) = already_selected.first().and_then(|first| first.player()) {
            player
        } else if let Some(card) = source {
            db[card].controller.into()
        } else {
            db.turn.priority_player()
        }
    }

    fn options(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        modes: &[usize],
    ) -> Options {
        self.pay_costs[self.paying as usize].options(db, source, already_selected, modes)
    }

    fn select(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
    ) -> SelectionResult {
        if option.is_none() && self.or_else.is_some() {
            self.apply_or_else = true;

            return SelectionResult::Complete;
        }

        if let SelectionResult::Complete =
            self.pay_costs[self.paying as usize].select(db, source, option, selected)
        {
            self.paying += 1;
        }

        if (self.paying as usize) == self.pay_costs.len() {
            SelectionResult::Complete
        } else {
            SelectionResult::PendingChoice
        }
    }

    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let mut results = vec![];

        if self.apply_or_else {
            for effect in self.or_else.mut_or_insert_default().effects.iter_mut() {
                results.extend(effect.effect.as_mut().unwrap().apply(
                    db,
                    source,
                    selected,
                    skip_replacement,
                ));
            }
        } else {
            for pay in self.pay_costs.iter_mut() {
                results.extend(pay.apply(db, source, selected, skip_replacement));
            }
        }

        results
    }
}
