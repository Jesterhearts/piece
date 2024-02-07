use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database, ExileReason},
    protogen::{
        effects::{
            ChooseCast, ClearSelected, Discover, Duration, MoveToBottomOfLibrary, PopSelected,
            PushSelected, SelectExiledWithCascadeOrDiscover, ShuffleSelected,
        },
        targets::Location,
    },
    stack::{Selected, TargetType},
};

impl EffectBehaviors for Discover {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let discover_value = self.count.count(db, source, selected);
        let source = source.unwrap();
        let owner = db[source].owner;

        let mut casting = vec![];
        while let Some(card) = db.all_players[owner].library.draw() {
            card.move_to_exile(
                db,
                source,
                Some(ExileReason::CascadeOrDiscover),
                Duration::PERMANENTLY,
            );

            if !card.is_land(db) && card.faceup_face(db).cost.cmc() < discover_value as usize {
                casting.push(Selected {
                    location: Some(Location::IN_EXILE),
                    target_type: TargetType::Card(card),
                    targeted: false,
                    restrictions: vec![],
                });
                break;
            }
        }

        let mut results = vec![EffectBundle {
            effects: vec![
                PushSelected::default().into(),
                ClearSelected::default().into(),
                SelectExiledWithCascadeOrDiscover::default().into(),
                ShuffleSelected::default().into(),
                MoveToBottomOfLibrary::default().into(),
                PopSelected::default().into(),
            ],
            source: Some(source),
            ..Default::default()
        }];

        results.push(EffectBundle {
            push_on_enter: Some(casting),
            effects: vec![
                ChooseCast {
                    discovering: true,
                    ..Default::default()
                }
                .into(),
                PopSelected::default().into(),
            ],
            source: Some(source),
            ..Default::default()
        });

        results
    }
}
