use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
};

#[test]
fn sacrifice_draw_gain_mana() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let attendant = CardId::upload(&mut db, &cards, player, "Darigaaz's Attendant");
    let results = Battlefield::add_from_stack(&mut db, attendant, vec![]);
    assert_eq!(results, []);

    let results = Battlefield::activate_ability(&mut db, &mut all_players, attendant, 0);
    assert_eq!(
        results,
        [
            UnresolvedActionResult::PermanentToGraveyard(attendant),
            UnresolvedActionResult::AddAbilityToStack {
                source: attendant,
                ability: attendant
                    .activated_abilities(&mut db)
                    .first()
                    .copied()
                    .unwrap(),
                valid_targets: Default::default(),
            }
        ]
    );

    let results = Battlefield::maybe_resolve(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    Ok(())
}
