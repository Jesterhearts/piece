use rand::{seq::SliceRandom, thread_rng};

use crate::{
    effects::{EffectBehaviors, EffectBundle, PendingEffects, SelectedStack},
    in_play::{CardId, Database, ExileReason},
    library::Library,
    protogen::{
        effects::{CastSelected, Discover, Effect, MoveToBottomOfLibrary},
        targets::Location,
    },
    stack::{Selected, TargetType},
};

impl EffectBehaviors for Discover {
    fn apply(
        &mut self,
        db: &mut Database,
        pending: &mut PendingEffects,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        let discover_value = self.count.count(db, source, selected);
        let source = source.unwrap();

        let mut casting = vec![];
        let mut exiled = vec![];
        while let Some(card) = Library::exile_top_card(
            db,
            db[source].owner,
            source,
            Some(ExileReason::CascadeOrDiscover),
        ) {
            if !card.is_land(db) && card.faceup_face(db).cost.cmc() < discover_value as usize {
                casting.push(Selected {
                    location: Some(Location::IN_EXILE),
                    target_type: TargetType::Card(card),
                    targeted: false,
                    restrictions: vec![],
                });
                break;
            }
            exiled.push(Selected {
                location: Some(Location::IN_EXILE),
                target_type: TargetType::Card(card),
                targeted: false,
                restrictions: vec![],
            });
        }

        pending.push_back(EffectBundle {
            selected: SelectedStack::new(casting),
            effects: vec![Effect {
                effect: Some(
                    CastSelected {
                        move_to_hand_if_not_cast: true,
                        ..Default::default()
                    }
                    .into(),
                ),
                ..Default::default()
            }],
            source: Some(source),
            ..Default::default()
        });

        exiled.shuffle(&mut thread_rng());
        pending.push_back(EffectBundle {
            selected: SelectedStack::new(exiled),
            effects: vec![Effect {
                effect: Some(MoveToBottomOfLibrary::default().into()),
                ..Default::default()
            }],
            source: Some(source),
            ..Default::default()
        })
    }
}
