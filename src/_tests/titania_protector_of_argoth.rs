use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::Database,
    in_play::{self, CardId, OnBattlefield},
    load_cards,
    player::AllPlayers,
    stack::Stack,
};

#[test]
fn etb() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_owned(), 20);
    all_players[player].infinite_mana();

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    land.move_to_graveyard(&mut db);

    let titania = CardId::upload(&mut db, &cards, player, "Titania, Protector of Argoth");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, titania);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(in_play::cards::<OnBattlefield>(&mut db), [titania, land]);

    Ok(())
}

#[test]
fn graveyard_trigger() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_owned(), 20);
    all_players[player].infinite_mana();

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    land.move_to_battlefield(&mut db);

    let titania = CardId::upload(&mut db, &cards, player, "Titania, Protector of Argoth");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, titania);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Battlefield::permanent_to_graveyard(&mut db, land);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(Battlefield::creatures(&mut db).len(), 2);

    Ok(())
}
