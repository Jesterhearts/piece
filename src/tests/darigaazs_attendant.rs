use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    in_play::CardId,
    load_cards,
    player::AllPlayers,
    prepare_db,
};

#[test]
fn sacrifice_draw_gain_mana() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let attendant = CardId::upload(&db, &cards, player, "Darigaaz's Attendant")?;
    let results = Battlefield::add_from_stack(&db, attendant, vec![])?;
    assert_eq!(results, []);

    let results = Battlefield::activate_ability(&db, &mut all_players, attendant, 0)?;
    assert_eq!(
        results,
        [
            UnresolvedActionResult::PermanentToGraveyard(attendant),
            UnresolvedActionResult::AddAbilityToStack {
                source: attendant,
                ability: attendant
                    .activated_abilities(&db)?
                    .first()
                    .copied()
                    .unwrap_or_default(),
                valid_targets: Default::default(),
            }
        ]
    );

    let results = Battlefield::maybe_resolve(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    Ok(())
}
