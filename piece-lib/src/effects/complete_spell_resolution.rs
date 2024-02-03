use crate::{
    effects::{ApplyResult, EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    log::Log,
    protogen::{
        effects::{CompleteSpellResolution, MoveToBattlefield, MoveToGraveyard, PopSelected},
        targets::Location,
    },
};

impl EffectBehaviors for CompleteSpellResolution {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        _selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let card = source.unwrap();
        Log::spell_resolved(db, card);

        if card.is_in_location(db, Location::IN_STACK) {
            let effects = if card.is_permanent(db) {
                vec![
                    MoveToBattlefield::default().into(),
                    PopSelected::default().into(),
                    PopSelected::default().into(),
                ]
            } else {
                vec![
                    MoveToGraveyard::default().into(),
                    PopSelected::default().into(),
                ]
            };

            vec![ApplyResult::PushFront(EffectBundle {
                effects,
                ..Default::default()
            })]
        } else {
            vec![]
        }
    }
}
