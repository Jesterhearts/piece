use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    in_play::{self, CardId, Database, OnBattlefield},
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    stack::Stack,
    turns::Turn,
};

#[test]
fn creates_tokens() -> anyhow::Result<()> {
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
    let turn = Turn::new(&mut db, &all_players);

    let card = CardId::upload(&mut db, &cards, player, "Forbidden Friendship");
    let targets = card.valid_targets(&mut db, &HashSet::default());
    card.move_to_stack(&mut db, targets, None, vec![]);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(in_play::cards::<OnBattlefield>(&mut db).len(), 2);

    Ok(())
}
