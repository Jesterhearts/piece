use crate::{
    effects::{EffectBehaviors, EffectBundle, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::{DestroySelected, Effect, MoveToGraveyard},
    stack::TargetType,
};

impl EffectBehaviors for DestroySelected {
    fn apply(
        &mut self,
        db: &mut Database,
        pending: &mut PendingEffects,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
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

        pending.push_back(EffectBundle {
            effects,
            source,
            ..Default::default()
        });
    }
}
