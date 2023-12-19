use std::collections::HashSet;

use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::{CardId, Database, InHand, InLibrary, OnBattlefield},
    load_cards,
    player::AllPlayers,
    stack::Stack,
    types::{Subtype, Type},
};

#[test]
fn cascades() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

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
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::move_card_to_stack_from_hand(&mut db, hand1, false);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve the first cascade
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose to cast
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose targets for metamorphosis.
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    // Resolve majestic metamorphosis
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(
        zhul.types(&mut db),
        IndexSet::from([Type::Artifact, Type::Creature, Type::Legendary])
    );
    assert_eq!(
        zhul.subtypes(&mut db),
        IndexSet::from([Subtype::Eldrazi, Subtype::Angel])
    );

    // Resolve the first cascade
    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Choose not to cast
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, None);
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
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(player.get_cards::<OnBattlefield>(&mut db), [zhul, hand1]);

    Ok(())
}
