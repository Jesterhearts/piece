use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, EffectBundle, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::{
        counters::Counter,
        effects::{Dest, Effect, Explore, MoveToGraveyard, MoveToTopOfLibrary, SelectDestinations},
        targets::Location,
        triggers::TriggerSource,
        types::Type,
    },
    stack::{Selected, Stack, TargetType},
    types::TypeSet,
};

impl EffectBehaviors for Explore {
    fn apply(
        &mut self,
        db: &mut Database,
        pending: &mut PendingEffects,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        let mut explored = vec![];

        let controller = db[source.unwrap()].controller;
        for target in selected
            .iter()
            .filter(|target| {
                matches!(target.location, Some(Location::ON_BATTLEFIELD))
                    && (!target.targeted || target.id(db).unwrap().can_be_targeted(db, controller))
            })
            .collect_vec()
        {
            let explorer = target.id(db).unwrap();

            if let Some(card) = db.all_players[controller].library.draw() {
                db[card].revealed = true;
                if card.types_intersect(db, &TypeSet::from([Type::LAND])) {
                    card.move_to_hand(db);
                } else {
                    *db[explorer].counters.entry(Counter::P1P1).or_default() += 1;
                    explored.push(Selected {
                        location: None,
                        target_type: TargetType::Card(card),
                        targeted: false,
                        restrictions: vec![],
                    });
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
                        pending.extend(Stack::move_trigger_to_stack(db, listener, trigger));
                    }
                });
        }

        pending.push_front(EffectBundle {
            selected: SelectedStack::new(explored),
            effects: vec![Effect {
                effect: Some(
                    SelectDestinations {
                        destinations: vec![
                            Dest {
                                count: 1,
                                destination: Some(MoveToGraveyard::default().into()),
                                ..Default::default()
                            },
                            Dest {
                                count: 1,
                                destination: Some(MoveToTopOfLibrary::default().into()),
                                ..Default::default()
                            },
                        ],
                        ..Default::default()
                    }
                    .into(),
                ),
                ..Default::default()
            }],
            source,
            ..Default::default()
        });
    }
}
