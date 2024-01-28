use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::effects::IfThenElse,
};

impl EffectBehaviors for IfThenElse {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let mut results = vec![];
        if selected.iter().all(|selected| match &selected.target_type {
            crate::stack::TargetType::Card(card) => {
                card.passes_restrictions(db, LogId::current(db), source.unwrap(), &self.if_)
            }
            crate::stack::TargetType::Stack(_) => todo!(),
            crate::stack::TargetType::Ability { .. } => todo!(),
            crate::stack::TargetType::ReplacementAbility(_) => todo!(),
            crate::stack::TargetType::Player(player) => player.passes_restrictions(
                db,
                LogId::current(db),
                db[source.unwrap()].controller,
                &self.if_,
            ),
        }) {
            for effect in self.then.iter_mut() {
                results.extend(effect.effect.as_mut().unwrap().apply(
                    db,
                    source,
                    selected,
                    skip_replacement,
                ));
            }
        } else {
            for effect in self.else_.iter_mut() {
                results.extend(effect.effect.as_mut().unwrap().apply(
                    db,
                    source,
                    selected,
                    skip_replacement,
                ));
            }
        }

        results
    }
}
