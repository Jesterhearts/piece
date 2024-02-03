use indexmap::IndexSet;
use pretty_assertions::assert_eq;
use protobuf::Enum;

use crate::{
    effects::{PendingEffects, SelectionResult},
    in_play::{CardId, CastFrom, Database},
    library::Library,
    load_cards,
    player::AllPlayers,
    protogen::{
        keywords::Keyword,
        types::{Subtype, Type},
    },
    stack::Stack,
    types::{SubtypeSet, TypeSet},
};

#[test]
fn reveals_clones() -> anyhow::Result<()> {
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
    let player1 = all_players.new_player("Player".to_string(), 20);
    let player2 = all_players.new_player("Player".to_string(), 20);
    let mut db = Database::new(all_players);

    let haunting = CardId::upload(&mut db, &cards, player1, "Haunting Imitation");
    let mut results = PendingEffects::default();
    results.apply_results(haunting.move_to_stack(&mut db, vec![], CastFrom::Hand, vec![]));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let land = CardId::upload(&mut db, &cards, player1, "Forest");
    let creature = CardId::upload(&mut db, &cards, player2, "Alpine Grizzly");

    Library::place_on_top(&mut db, player1, land);
    Library::place_on_top(&mut db, player2, creature);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let on_battlefield = &mut db.battlefield[player1];
    assert_eq!(on_battlefield.len(), 1);
    let token = on_battlefield.pop().unwrap();

    assert_eq!(db[token].modified_types, TypeSet::from([Type::CREATURE]));
    assert_eq!(
        db[token].modified_subtypes,
        SubtypeSet::from([Subtype::BEAR, Subtype::SPIRIT])
    );
    assert_eq!(token.power(&db), Some(1));
    assert_eq!(token.toughness(&db), Some(1));
    assert_eq!(
        db[token].modified_keywords,
        [(Keyword::FLYING.value(), 1)].into_iter().collect()
    );

    Ok(())
}

#[test]
fn no_reveals_returns_to_hand() -> anyhow::Result<()> {
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
    let player1 = all_players.new_player("Player".to_string(), 20);
    let player2 = all_players.new_player("Player".to_string(), 20);
    let mut db = Database::new(all_players);

    let haunting = CardId::upload(&mut db, &cards, player1, "Haunting Imitation");
    let mut results = PendingEffects::default();
    results.apply_results(haunting.move_to_stack(&mut db, vec![], CastFrom::Hand, vec![]));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let land1 = CardId::upload(&mut db, &cards, player1, "Forest");
    let land2 = CardId::upload(&mut db, &cards, player2, "Swamp");

    Library::place_on_top(&mut db, player1, land1);
    Library::place_on_top(&mut db, player2, land2);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(db.battlefield[player1], IndexSet::<CardId>::default());
    assert_eq!(db.hand[player1], IndexSet::from([haunting]));

    Ok(())
}
