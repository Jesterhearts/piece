use crate::{
    effects::{EffectBehaviors, EffectBundle, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    protogen::effects::{CastSelected, Effect, MoveToStack},
};

impl EffectBehaviors for CastSelected {
    fn apply(
        &mut self,
        db: &mut Database,
        pending: &mut PendingEffects,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        for target in selected.iter() {
            let card = target.id(db).unwrap();

            let mut to_cast = card.faceup_face(db).to_cast.clone();
            to_cast.push(Effect {
                effect: Some(MoveToStack::default().into()),
                ..Default::default()
            });
            pending.push_back(EffectBundle {
                selected: SelectedStack::new(vec![target.clone()]),
                effects: to_cast,
                source: Some(card),
                ..Default::default()
            });
        }
    }
}
