use crate::{
    action_result::Action,
    in_play::Database,
    log::LogId,
    pending_results::PendingResults,
    protogen::{counters::Counter, triggers::TriggerSource, types::Type},
    stack::{ActiveTarget, Stack},
    types::TypeSet,
};

#[derive(Debug, Clone)]
pub(crate) struct Explore {
    pub(crate) target: ActiveTarget,
}

impl Action for Explore {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { target } = self;
        let explorer = target.id(db).unwrap();
        let controller = db[explorer].controller;
        let mut results = PendingResults::default();

        if let Some(card) = db.all_players[controller].library.draw() {
            db[card].revealed = true;
            if card.types_intersect(db, &TypeSet::from([Type::LAND])) {
                card.move_to_hand(db);
            } else {
                *db[explorer].counters.entry(Counter::P1P1).or_default() += 1;
                results.push_choose_library_or_graveyard(card);
            }
        } else {
            *db[explorer].counters.entry(Counter::P1P1).or_default() += 1;
        }

        db.active_triggers_of_source(TriggerSource::CREATURE_EXPLORES)
            .into_iter()
            .for_each(|(listener, trigger)| {
                if explorer.passes_restrictions(
                    db,
                    LogId::current(db),
                    listener,
                    &trigger.trigger.restrictions,
                ) {
                    results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
                }
            });

        results
    }
}
