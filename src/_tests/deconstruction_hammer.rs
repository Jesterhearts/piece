use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield,
    in_play::{self, Database, OnBattlefield},
    in_play::{CardId, InGraveyard},
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    stack::Stack,
    turns::{Phase, Turn},
};

#[test]
fn destroys_artifact() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .pretty()
        .with_ansi(false)
        .with_line_number(true)
        .with_file(true)
        .with_target(false)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ENTER)
        .with_writer(std::io::stderr)
        .try_init();

    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player1 = all_players.new_player(String::default(), 20);
    all_players[player1].infinite_mana();
    let player2 = all_players.new_player(String::default(), 20);

    let mut turn = Turn::new(&mut db, &all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player1, "Alpine Grizzly");
    let card2 = CardId::upload(&mut db, &cards, player1, "Deconstruction Hammer");
    let card3 = CardId::upload(&mut db, &cards, player2, "Abzan Banner");

    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card2, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card3, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Equip the bear
    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, &None, player1, card2, 0);
    // Pay the costs
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // End pay costs
    // Target the bear
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve the equip
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Activate the ability on the bear, targeting the banner
    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, &None, player1, card, 0);
    // Pay the generic mana
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose the default only target
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve the ability
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(in_play::cards::<OnBattlefield>(&mut db), [card]);
    assert_eq!(player1.get_cards::<InGraveyard>(&mut db), [card2]);
    assert_eq!(player2.get_cards::<InGraveyard>(&mut db), [card3]);

    Ok(())
}
