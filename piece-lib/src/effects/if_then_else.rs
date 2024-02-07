use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
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
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
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
            vec![EffectBundle {
                source,
                effects: self.then.clone(),
                ..Default::default()
            }]
        } else {
            vec![EffectBundle {
                source,
                effects: self.else_.clone(),
                ..Default::default()
            }]
        }
    }
}
