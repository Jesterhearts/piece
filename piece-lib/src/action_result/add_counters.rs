use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    log::LogId,
    pending_results::PendingResults,
    protogen::{
        counters::Counter,
        effects::{self, count::dynamic::Dynamic},
    },
};

#[derive(Debug, Clone)]
pub(crate) struct AddCounters {
    pub(crate) source: CardId,
    pub(crate) target: CardId,
    pub(crate) count: effects::count::Count,
    pub(crate) counter: protobuf::EnumOrUnknown<Counter>,
}

impl Action for AddCounters {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            source,
            target,
            count,
            counter,
        } = self;

        match count {
            effects::count::Count::Fixed(count) => {
                *db[*target]
                    .counters
                    .entry(counter.enum_value().unwrap())
                    .or_default() += count.count as usize;
            }
            effects::count::Count::Dynamic(dynamic) => match dynamic.dynamic.as_ref().unwrap() {
                Dynamic::X(_) => {
                    let x = source.get_x(db);
                    if x > 0 {
                        *db[*target]
                            .counters
                            .entry(counter.enum_value().unwrap())
                            .or_default() += x;
                    }
                }
                Dynamic::LeftBattlefieldThisTurn(left) => {
                    let cards = CardId::left_battlefield_this_turn(db);
                    let x = cards
                        .filter(|card| {
                            card.passes_restrictions(
                                db,
                                LogId::current(db),
                                *source,
                                &left.restrictions,
                            )
                        })
                        .count();
                    if x > 0 {
                        *db[*target]
                            .counters
                            .entry(counter.enum_value().unwrap())
                            .or_default() += x;
                    }
                }
            },
        }

        PendingResults::default()
    }
}
