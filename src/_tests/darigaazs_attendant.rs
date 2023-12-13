use std::collections::VecDeque;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield, PendingResult, PendingResults, ResolutionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    mana::Mana,
    player::AllPlayers,
};

#[test]
fn sacrifice_gain_mana() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let attendant = CardId::upload(&mut db, &cards, player, "Darigaaz's Attendant");
    let results = Battlefield::add_from_stack_or_hand(&mut db, attendant, vec![]);
    assert_eq!(results, PendingResults::default());

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, attendant, 0);
    assert_eq!(
        results,
        PendingResults {
            results: VecDeque::from([PendingResult {
                apply_immediately: vec![
                    ActionResult::PermanentToGraveyard(attendant),
                    ActionResult::GainMana {
                        gain: vec![Mana::Black, Mana::Red, Mana::Green],
                        target: player.into(),
                    }
                ],
                then_resolve: Default::default(),
                recompute: false
            }])
        }
    );

    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    Ok(())
}
