use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::effects::Unless,
};

impl EffectBehaviors for Unless {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        if !selected.iter().any(|selected| match &selected.target_type {
            crate::stack::TargetType::Card(card) => {
                card.passes_restrictions(db, LogId::current(db), source.unwrap(), &self.unless)
            }
            crate::stack::TargetType::Stack(_) => todo!(),
            crate::stack::TargetType::Ability { .. } => todo!(),
            crate::stack::TargetType::ReplacementAbility(_) => todo!(),
            crate::stack::TargetType::Player(player) => player.passes_restrictions(
                db,
                LogId::current(db),
                db[source.unwrap()].controller,
                &self.unless,
            ),
        }) {
            vec![EffectBundle {
                source,
                effects: self.then.clone(),
                ..Default::default()
            }]
        } else {
            vec![]
        }
    }
}
