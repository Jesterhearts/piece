use pretty_assertions::assert_eq;

use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectionResult},
    in_play::CardId,
    in_play::Database,
    library::Library,
    load_cards,
    player::AllPlayers,
    protogen::{effects::MoveToBattlefield, targets::Location},
    stack::{Selected, Stack, TargetType},
};

#[test]
fn etb() -> anyhow::Result<()> {
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

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let mut db = Database::new(all_players);

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    let nonland = CardId::upload(&mut db, &cards, player, "Annul");

    Library::place_on_top(&mut db, player, land);
    Library::place_on_top(&mut db, player, nonland);

    let glowspore = CardId::upload(&mut db, &cards, player, "Glowspore Shaman");
    let mut results = PendingEffects::default();
    results.selected.push(Selected {
        location: Some(Location::ON_BATTLEFIELD),
        target_type: TargetType::Card(glowspore),
        targeted: false,
        restrictions: vec![],
    });
    let to_apply = MoveToBattlefield::default().apply(&mut db, None, &mut results.selected, false);
    results.apply_results(to_apply);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert!(db.stack.is_empty());
    assert_eq!(db.all_players[player].library.len(), 1);

    Ok(())
}
