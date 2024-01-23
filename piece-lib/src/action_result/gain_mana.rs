use crate::{
    action_result::Action,
    in_play::Database,
    pending_results::PendingResults,
    player::Controller,
    protogen::mana::{Mana, ManaRestriction, ManaSource},
};

#[derive(Debug, Clone)]
pub(crate) struct GainMana {
    pub(crate) gain: Vec<protobuf::EnumOrUnknown<Mana>>,
    pub(crate) target: Controller,
    pub(crate) source: protobuf::EnumOrUnknown<ManaSource>,
    pub(crate) restriction: protobuf::EnumOrUnknown<ManaRestriction>,
}

impl Action for GainMana {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            gain,
            target,
            source,
            restriction,
        } = self;
        for mana in gain {
            db.all_players[*target].mana_pool.apply(
                mana.enum_value().unwrap(),
                source.enum_value().unwrap(),
                restriction.enum_value().unwrap(),
            )
        }
        PendingResults::default()
    }
}
