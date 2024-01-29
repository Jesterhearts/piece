use rand::{seq::SliceRandom, thread_rng};

use crate::{
    effects::{ApplyResult, EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database, ExileReason},
    library::Library,
    protogen::{
        effects::{Cascade, CastSelected, MoveToBottomOfLibrary, PopSelected},
        targets::Location,
    },
    stack::{Selected, TargetType},
};

impl EffectBehaviors for Cascade {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        _selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let source = source.unwrap();
        let mana_value = db[source].modified_cost.cmc() + source.get_x(db);

        let mut casting = vec![];
        let mut exiled = vec![];
        while let Some(card) = Library::exile_top_card(
            db,
            db[source].owner,
            source,
            Some(ExileReason::CascadeOrDiscover),
        ) {
            if !card.is_land(db) && card.faceup_face(db).cost.cmc() < mana_value {
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

        let mut results = vec![ApplyResult::PushBack(EffectBundle {
            push_on_enter: Some(casting),
            effects: vec![
                CastSelected::default().into(),
                PopSelected::default().into(),
            ],
            source: Some(source),
            ..Default::default()
        })];

        exiled.shuffle(&mut thread_rng());
        results.push(ApplyResult::PushBack(EffectBundle {
            push_on_enter: Some(exiled),
            effects: vec![
                MoveToBottomOfLibrary::default().into(),
                PopSelected::default().into(),
            ],
            source: Some(source),
            ..Default::default()
        }));

        results
    }
}
