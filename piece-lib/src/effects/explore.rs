use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::{
        counters::Counter,
        effects::{
            Dest, Explore, MoveToGraveyard, MoveToTopOfLibrary, PopSelected, SelectDestinations,
        },
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
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let mut explored = vec![];

        let mut results = vec![];
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
                        results.push(Stack::move_trigger_to_stack(db, listener, trigger));
                    }
                });
        }

        selected.save();
        selected.clear();
        selected.extend(explored);

        results.push(EffectBundle {
            effects: vec![
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
                PopSelected::default().into(),
            ],
            source,
            ..Default::default()
        });

        results
    }
}
