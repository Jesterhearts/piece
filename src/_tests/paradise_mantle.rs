use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::Stack,
};

#[test]
fn adds_ability() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let equipment = CardId::upload(&mut db, &cards, player, "Paradise Mantle");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, equipment, vec![]);

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, creature, vec![]);

    assert_eq!(creature.activated_abilities(&mut db), []);

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, equipment, 0);
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(creature.activated_abilities(&mut db).len(), 1);

    Ok(())
}
