use std::collections::HashSet;

use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield,
    in_play::{CardId, Database, InHand, InLibrary, OnBattlefield},
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    stack::Stack,
    turns::Turn,
    types::{Subtype, Type},
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
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();
    let turn = Turn::new(&mut db, &all_players);

    let hand1 = CardId::upload(&mut db, &cards, player, "Hexplate Golem");
    hand1.move_to_hand(&mut db);

    let deck1 = CardId::upload(&mut db, &cards, player, "Majestic Metamorphosis");
    all_players[player].deck.place_on_top(&mut db, deck1);
    let deck2 = CardId::upload(&mut db, &cards, player, "Forest");
    all_players[player].deck.place_on_top(&mut db, deck2);
    let deck3 = CardId::upload(&mut db, &cards, player, "Majestic Metamorphosis");
    all_players[player].deck.place_on_top(&mut db, deck3);
    let deck4 = CardId::upload(&mut db, &cards, player, "Forest");
    all_players[player].deck.place_on_top(&mut db, deck4);

    let zhul = CardId::upload(&mut db, &cards, player, "Zhulodok, Void Gorger");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, zhul, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::move_card_to_stack_from_hand(&mut db, hand1, false);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve the first cascade
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose to cast
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose targets for metamorphosis.
    let result = results.resolve(&mut db, &mut all_players, &turn, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve majestic metamorphosis
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(
        zhul.types(&db),
        IndexSet::from([Type::Artifact, Type::Creature, Type::Legendary])
    );
    assert_eq!(
        zhul.subtypes(&db),
        IndexSet::from([Subtype::Eldrazi, Subtype::Angel])
    );

    // Resolve the first cascade
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose not to cast
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(
        player
            .get_cards::<InLibrary>(&mut db)
            .into_iter()
            .collect::<HashSet<_>>(),
        HashSet::from([deck1, deck4])
    );

    assert_eq!(
        player
            .get_cards::<InHand>(&mut db)
            .into_iter()
            .collect::<HashSet<_>>(),
        HashSet::from([deck2])
    );

    // Resolve the actual golem
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(player.get_cards::<OnBattlefield>(&mut db), [zhul, hand1]);

    Ok(())
}
