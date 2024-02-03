use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::CastSelected,
    stack::Stack,
};

impl EffectBehaviors for CastSelected {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let mut results = vec![];
        for target in selected.iter() {
            let card = target.id(db).unwrap();
            results.push(Stack::prepare_card_for_stack(db, card, self.pay_costs));
        }

        results
    }
}
