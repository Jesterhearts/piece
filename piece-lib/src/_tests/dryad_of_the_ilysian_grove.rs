use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    in_play::Database,
    load_cards,
    pending_results::ResolutionResult,
    player::{AllPlayers, Player},
    protogen::ids::CardId,
    protogen::types::Subtype,
    types::SubtypeSet,
};

#[test]
fn adds_land_types() -> anyhow::Result<()> {
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
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, &land, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let card = CardId::upload(&mut db, &cards, player, "Dryad of the Ilysian Grove");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, &card, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(Player::lands_per_turn(&mut db, player), 2);

    assert_eq!(
        db[&land].modified_subtypes,
        SubtypeSet::from([
            Subtype::PLAINS,
            Subtype::ISLAND,
            Subtype::SWAMP,
            Subtype::MOUNTAIN,
            Subtype::FOREST,
        ])
    );

    Ok(())
}
