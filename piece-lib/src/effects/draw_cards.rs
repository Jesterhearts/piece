use crate::{
    effects::{handle_replacements, EffectBehaviors, EffectBundle, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::effects::{replacement_effect::Replacing, DrawCards, Effect, PlayerLoses},
};

impl EffectBehaviors for DrawCards {
    fn apply(
        &mut self,
        db: &mut Database,
        pending: &mut PendingEffects,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        skip_replacement: bool,
    ) {
        let target = selected.first().unwrap().player().unwrap();
        for _ in 0..self.count.count(db, source, selected) {
            if skip_replacement {
                if let Some(card) = db.all_players[target].library.draw() {
                    card.move_to_hand(db);
                } else {
                    pending.push_back(EffectBundle {
                        selected: selected.clone(),
                        effects: vec![Effect {
                            effect: Some(PlayerLoses::default().into()),
                            ..Default::default()
                        }],
                        source,
                        ..Default::default()
                    })
                }
            } else {
                handle_replacements(
                    db,
                    pending,
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
                );
            }
        }
    }
}
