use itertools::Itertools;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::{CardId, Database, InGraveyard, InHand},
    load_cards,
    mana::{Mana, ManaRestriction},
    player::{mana_pool::ManaSource, AllPlayers},
    stack::Stack,
    turns::{Phase, Turn},
};

#[test]
fn sacrifice_draw_card() -> anyhow::Result<()> {
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
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    all_players[player].deck.place_on_top(&mut db, land);

    let mut turn = Turn::new(&mut db, &all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, player, card, 1);
    // Pay banner cost
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // End pay banner costs
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(player.get_cards::<InGraveyard>(&mut db), vec![card]);
    assert_eq!(player.get_cards::<InHand>(&mut db), vec![land]);

    Ok(())
}

#[test]
fn add_mana() -> anyhow::Result<()> {
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
    let player = all_players.new_player("Player".to_string(), 20);
    let mut turn = Turn::new(&mut db, &all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, player, card, 0);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::PendingChoice);

    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(
        all_players[player].mana_pool.all_mana().collect_vec(),
        [
            (1, Mana::White, ManaSource::Any, ManaRestriction::None),
            (0, Mana::Blue, ManaSource::Any, ManaRestriction::None),
            (0, Mana::Black, ManaSource::Any, ManaRestriction::None),
            (0, Mana::Red, ManaSource::Any, ManaRestriction::None),
            (0, Mana::Green, ManaSource::Any, ManaRestriction::None),
            (0, Mana::Colorless, ManaSource::Any, ManaRestriction::None),
        ]
    );

    Ok(())
}
