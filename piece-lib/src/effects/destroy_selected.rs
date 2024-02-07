use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::{DestroySelected, Effect, MoveToGraveyard},
    stack::TargetType,
};

impl EffectBehaviors for DestroySelected {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let mut effects = vec![];
        for target in selected.iter() {
            let TargetType::Card(card) = target.target_type else {
                unreachable!()
            };

            if !card.indestructible(db) {
                effects.push(Effect {
                    effect: Some(MoveToGraveyard::default().into()),
                    ..Default::default()
                })
            }
        }

        vec![EffectBundle {
            effects,
            source,
            ..Default::default()
        }]
    }
}
