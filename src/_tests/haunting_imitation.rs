use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::ResolutionResult,
    card::Keyword,
    in_play::{CardId, Database, InHand, OnBattlefield},
    load_cards,
    player::AllPlayers,
    stack::Stack,
    types::{Subtype, Type},
};

#[test]
fn reveals_clones() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut all_players = AllPlayers::default();
    let player1 = all_players.new_player("Player".to_string(), 20);
    let player2 = all_players.new_player("Player".to_string(), 20);

    let mut db = Database::default();

    let haunting = CardId::upload(&mut db, &cards, player1, "Haunting Imitation");
    let targets = haunting.valid_targets(&mut db);
    haunting.move_to_stack(&mut db, targets, None);

    let land = CardId::upload(&mut db, &cards, player1, "Forest");
    let creature = CardId::upload(&mut db, &cards, player2, "Alpine Grizzly");

    all_players[player1].deck.place_on_top(&mut db, land);
    all_players[player2].deck.place_on_top(&mut db, creature);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut on_battlefield = player1.get_cards::<OnBattlefield>(&mut db);
    assert_eq!(on_battlefield.len(), 1);
    let token = on_battlefield.pop().unwrap();

    assert_eq!(token.types(&db), IndexSet::from([Type::Creature]));
    assert_eq!(
        token.subtypes(&db),
        IndexSet::from([Subtype::Bear, Subtype::Spirit])
    );
    assert_eq!(token.power(&db), Some(1));
    assert_eq!(token.toughness(&db), Some(1));
    assert_eq!(token.keywords(&db), [Keyword::Flying].into_iter().collect());

    Ok(())
}

#[test]
fn no_reveals_returns_to_hand() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut all_players = AllPlayers::default();
    let player1 = all_players.new_player("Player".to_string(), 20);
    let player2 = all_players.new_player("Player".to_string(), 20);

    let mut db = Database::default();

    let haunting = CardId::upload(&mut db, &cards, player1, "Haunting Imitation");
    let mut results = Stack::move_card_to_stack_from_hand(&mut db, haunting, false);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let land1 = CardId::upload(&mut db, &cards, player1, "Forest");
    let land2 = CardId::upload(&mut db, &cards, player2, "Swamp");

    all_players[player1].deck.place_on_top(&mut db, land1);
    all_players[player2].deck.place_on_top(&mut db, land2);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(player1.get_cards::<OnBattlefield>(&mut db).len(), 0);
    assert_eq!(player1.get_cards::<InHand>(&mut db), vec![haunting]);

    Ok(())
}
