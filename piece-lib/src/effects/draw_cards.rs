use crate::{
    effects::{handle_replacements, ApplyResult, EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::effects::{replacement_effect::Replacing, DrawCards, Effect, PlayerLoses},
    stack::{Selected, TargetType},
};

impl EffectBehaviors for DrawCards {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let mut results = vec![];
        let target = selected.first().unwrap().player().unwrap();
        for _ in 0..self.count.count(db, source, selected) {
            if skip_replacement {
                if let Some(card) = db.all_players[target].library.draw() {
                    card.move_to_hand(db);
                } else {
                    results.push(ApplyResult::PushBack(EffectBundle {
                        push_on_enter: Some(vec![Selected {
                            location: None,
                            target_type: TargetType::Player(target),
                            targeted: false,
                            restrictions: vec![],
                        }]),
                        effects: vec![Effect {
                            effect: Some(PlayerLoses::default().into()),
                            ..Default::default()
                        }],
                        source,
                        ..Default::default()
                    }));
                }
            } else {
                results.extend(handle_replacements(
                    db,
                    source,
                    Replacing::DRAW,
                    self.clone(),
                    |source, restrictions| {
                        target.passes_restrictions(
                            db,
                            LogId::current(db),
                            db[source].controller,
                            restrictions,
                        )
                    },
                ));
            }
        }

        results
    }
}
