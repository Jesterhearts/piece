use crate::{
    action_result::{
        cascade_exile_to_bottom_of_library::CascadeExileToBottomOfLibrary, Action, ActionResult,
    },
    in_play::{CardId, CastFrom, Database, ExileReason},
    library::Library,
    pending_results::PendingResults,
    player::Controller,
};

#[derive(Debug, Clone)]
pub(crate) struct Cascade {
    pub(crate) source: CardId,
    pub(crate) cascading: usize,
    pub(crate) player: Controller,
}

impl Action for Cascade {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            source,
            cascading,
            player,
        } = self;
        let mut results = PendingResults::default();
        results.cast_from(Some(CastFrom::Exile));

        while let Some(card) =
            Library::exile_top_card(db, (*player).into(), *source, Some(ExileReason::Cascade))
        {
            if !card.is_land(db) && card.faceup_face(db).cost.cmc() < *cascading {
                results.push_choose_cast(card, false, false);
                break;
            }
        }

        results.push_settled(ActionResult::from(CascadeExileToBottomOfLibrary {
            player: *player,
        }));

        results
    }
}
