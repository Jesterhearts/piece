use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::{CardId, Database},
    load_cards,
    player::AllPlayers,
    stack::Stack,
    turns::Turn,
};

#[test]
fn ability() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("name".to_string(), 20);
    all_players[player].infinite_mana();

    let turn = Turn::new(&all_players);

    let card = CardId::upload(&mut db, &cards, player, "Deadapult");
    card.move_to_battlefield(&mut db);

    let sac = CardId::upload(&mut db, &cards, player, "Blood Scrivener");
    sac.move_to_battlefield(&mut db);

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    bear.move_to_battlefield(&mut db);

    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, player, card, 0);
    let result = results.resolve(&mut db, &mut all_players, Some(1));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(bear.marked_damage(&db), 2);

    Ok(())
}
