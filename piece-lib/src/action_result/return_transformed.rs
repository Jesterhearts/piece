use crate::{
    action_result::Action,
    battlefield::{complete_add_from_exile, complete_add_from_graveyard, move_card_to_battlefield},
    in_play::{CardId, Database},
    pending_results::PendingResults,
    protogen::targets::Location,
};

#[derive(Debug, Clone)]
pub(crate) struct ReturnTransformed {
    pub(crate) target: CardId,
    pub(crate) enters_tapped: bool,
}

impl Action for ReturnTransformed {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            target,
            enters_tapped,
        } = self;
        target.transform(db);
        let mut results = PendingResults::default();
        let location = if target.is_in_location(db, Location::IN_EXILE) {
            Location::IN_EXILE
        } else if target.is_in_location(db, Location::IN_GRAVEYARD) {
            Location::IN_GRAVEYARD
        } else {
            unreachable!("unexpected location {:?}", target.target_from_location(db))
        };
        move_card_to_battlefield(db, *target, *enters_tapped, &mut results, None);
        match location {
            Location::IN_EXILE => complete_add_from_exile(db, *target, &mut results),
            Location::IN_GRAVEYARD => complete_add_from_graveyard(db, *target, &mut results),
            _ => unreachable!(),
        }

        results
    }
}
