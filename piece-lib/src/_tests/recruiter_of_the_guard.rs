use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields, in_play::Database, library::Library, load_cards,
    pending_results::ResolutionResult, player::AllPlayers, protogen::ids::CardId, stack::Stack,
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
    all_players[&player].infinite_mana();

    let mut db = Database::new(all_players);

    let bear = CardId::upload(&mut db, &cards, player.clone(), "Alpine Grizzly");
    Library::place_on_top(&mut db, &player, bear.clone());

    let spell = CardId::upload(&mut db, &cards, player.clone(), "Annul");
    Library::place_on_top(&mut db, &player, spell);

    let elesh = CardId::upload(
        &mut db,
        &cards,
        player.clone(),
        "Elesh Norn, Grand Cenobite",
    );
    Library::place_on_top(&mut db, &player, elesh);

    let recruiter = CardId::upload(&mut db, &cards, player.clone(), "Recruiter of the Guard");
    recruiter.move_to_hand(&mut db);
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, &recruiter, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(db.all_players[&player].library.len(), 2);

    assert_eq!(db.hand[&player], IndexSet::from([bear]));

    Ok(())
}
