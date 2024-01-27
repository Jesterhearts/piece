mod exile_cards_sharing_type;
mod exile_permanents;
mod exile_permanents_cmc_x;
mod pay_cost;
mod pay_mana;
mod sacrifice_permanent;
mod tap_permanent;
mod tap_permanents_power_x_or_more;

use crate::{
    effects::{ApplyResult, EffectBehaviors, Options, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
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
        modes: &mut Vec<usize>,
    ) -> SelectionResult {
        if let SelectionResult::Complete =
            self.pay_costs[self.paying as usize].select(db, source, option, selected, modes)
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
        modes: &[usize],
        skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let mut results = vec![];
        for pay in self.pay_costs.iter_mut() {
            results.extend(pay.apply(db, source, selected, modes, skip_replacement));
        }

        results
    }
}
