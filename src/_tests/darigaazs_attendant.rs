use std::collections::VecDeque;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield, PendingResult, PendingResults, ResolutionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    mana::Mana,
    player::AllPlayers,
    turns::{Phase, Turn},
};

#[test]
fn sacrifice_gain_mana() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();
    let mut turn = Turn::new(&all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let attendant = CardId::upload(&mut db, &cards, player, "Darigaaz's Attendant");
    let results = Battlefield::add_from_stack_or_hand(&mut db, attendant);
    assert_eq!(results, PendingResults::default());

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, &turn, attendant, 0);
    assert_eq!(
        results,
        PendingResults {
            results: VecDeque::from([PendingResult {
                apply_immediately: vec![],
                to_resolve: Default::default(),
                then_apply: vec![
                    ActionResult::PermanentToGraveyard(attendant),
                    ActionResult::SpendMana(player.into(), vec![Mana::Generic(1)]),
                    ActionResult::GainMana {
                        gain: vec![Mana::Black, Mana::Red, Mana::Green],
                        target: player.into(),
                    }
                ],
                recompute: false
            }]),
            applied: false,
        }
    );

    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    Ok(())
}
