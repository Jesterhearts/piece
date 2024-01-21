use std::collections::HashSet;

use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    in_play::Database,
    library::Library,
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    protogen::{
        ids::CardId,
        types::{Subtype, Type},
    },
    stack::Stack,
    types::{SubtypeSet, TypeSet},
};

#[test]
fn cascades() -> anyhow::Result<()> {
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

    let hand1 = CardId::upload(&mut db, &cards, player.clone(), "Hexplate Golem");
    hand1.move_to_hand(&mut db);

    let deck1 = CardId::upload(&mut db, &cards, player.clone(), "Majestic Metamorphosis");
    Library::place_on_top(&mut db, &player, deck1.clone());
    let deck2 = CardId::upload(&mut db, &cards, player.clone(), "Forest");
    Library::place_on_top(&mut db, &player, deck2.clone());
    let deck3 = CardId::upload(&mut db, &cards, player.clone(), "Majestic Metamorphosis");
    Library::place_on_top(&mut db, &player, deck3.clone());
    let deck4 = CardId::upload(&mut db, &cards, player.clone(), "Forest");
    Library::place_on_top(&mut db, &player, deck4.clone());

    let zhul = CardId::upload(&mut db, &cards, player.clone(), "Zhulodok, Void Gorger");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, &zhul, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::move_card_to_stack_from_hand(&mut db, hand1.clone(), false);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve the first cascade
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose to cast
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose targets for metamorphosis.
    let result = results.resolve(&mut db, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve majestic metamorphosis
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(
        db[&zhul].modified_types,
        TypeSet::from([Type::ARTIFACT, Type::CREATURE, Type::LEGENDARY])
    );
    assert_eq!(
        db[&zhul].modified_subtypes,
        SubtypeSet::from([Subtype::ELDRAZI, Subtype::ANGEL])
    );

    // Resolve the first cascade
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose not to cast
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(
        db.all_players[&player]
            .library
            .cards
            .iter()
            .cloned()
            .collect::<HashSet<_>>(),
        HashSet::from([deck1, deck4])
    );

    assert_eq!(db.hand[&player], IndexSet::from([deck2]));

    // Resolve the actual golem
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(db.battlefield[&player], IndexSet::from([zhul, hand1]));

    Ok(())
}
