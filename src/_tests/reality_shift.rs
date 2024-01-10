use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    in_play::CardId,
    in_play::Database,
    library::Library,
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    protogen::types::subtype::Subtype,
    stack::{ActiveTarget, Stack},
    types::SubtypeSet,
};

#[test]
fn resolves_shift() -> anyhow::Result<()> {
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

    let all_cards = load_cards()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let mut db = Database::new(all_players);

    let bear1 = CardId::upload(&mut db, &all_cards, player, "Alpine Grizzly");
    let bear2 = CardId::upload(&mut db, &all_cards, player, "Alpine Grizzly");
    let bear3 = CardId::upload(&mut db, &all_cards, player, "Alpine Grizzly");

    let mut results = Battlefields::add_from_stack_or_hand(&mut db, bear1, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, bear2, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    Library::place_on_top(&mut db, player, bear3);

    let shift = CardId::upload(&mut db, &all_cards, player, "Reality Shift");
    let mut results = shift.move_to_stack(
        &mut db,
        vec![vec![ActiveTarget::Battlefield { id: bear1 }]],
        None,
        vec![],
    );
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(db.exile[player], IndexSet::from([bear1]));

    assert_eq!(bear2.power(&db), Some(4));
    assert_eq!(bear2.toughness(&db), Some(2));
    assert_eq!(
        db[bear2].modified_subtypes,
        SubtypeSet::from([Subtype::Bear(Default::default())])
    );

    assert_eq!(bear3.power(&db), Some(2));
    assert_eq!(bear3.toughness(&db), Some(2));
    assert_eq!(db[bear3].modified_subtypes, SubtypeSet::from([]));

    Ok(())
}
