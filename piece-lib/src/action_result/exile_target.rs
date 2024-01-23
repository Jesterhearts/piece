use crate::{
    action_result::Action,
    battlefield::Battlefields,
    in_play::{CardId, Database, ExileReason},
    pending_results::PendingResults,
    protogen::{effects::Duration, targets::Location},
    stack::ActiveTarget,
};

#[derive(Debug, Clone)]
pub(crate) struct ExileTarget {
    pub(crate) source: CardId,
    pub(crate) target: ActiveTarget,
    pub(crate) duration: protobuf::EnumOrUnknown<Duration>,
    pub(crate) reason: Option<ExileReason>,
}

impl Action for ExileTarget {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            source,
            target,
            duration,
            reason,
        } = self;
        let Some(target) = target.id(db) else {
            unreachable!()
        };
        if let Duration::UNTIL_SOURCE_LEAVES_BATTLEFIELD = duration.enum_value().unwrap() {
            if !source.is_in_location(db, Location::ON_BATTLEFIELD) {
                return PendingResults::default();
            }
        }

        Battlefields::exile(db, *source, target, *reason, duration.enum_value().unwrap())
    }
}
