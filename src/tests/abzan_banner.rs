use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    in_play::{CardId, Database},
    load_cards,
    player::AllPlayers,
};

#[test]
fn sacrifice_draw_card() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let card = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let results = Battlefield::add_from_stack(&mut db, card, vec![]);
    assert_eq!(results, []);

    let results = Battlefield::activate_ability(&mut db, &mut all_players, card, 0);
    assert_eq!(
        results,
        [
            UnresolvedActionResult::TapPermanent(card),
            UnresolvedActionResult::PermanentToGraveyard(card),
            UnresolvedActionResult::AddAbilityToStack {
                source: card,
                ability: card.activated_abilities(&mut db).first().copied().unwrap(),
                valid_targets: Default::default(),
            }
        ]
    );

    let results = Battlefield::maybe_resolve(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    Ok(())
}

#[test]
fn add_mana() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let card = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let results = Battlefield::add_from_stack(&mut db, card, vec![]);
    assert_eq!(results, []);

    let results = Battlefield::activate_ability(&mut db, &mut all_players, card, 1);

    assert_eq!(
        results,
        [
            UnresolvedActionResult::TapPermanent(card),
            UnresolvedActionResult::GainMana {
                source: card,
                ability: card.activated_abilities(&mut db).last().copied().unwrap(),
                mode: None,
            }
        ]
    );

    let results = Battlefield::maybe_resolve(&mut db, &mut all_players, results);
    assert_eq!(
        results,
        [UnresolvedActionResult::GainMana {
            source: card,
            ability: card.activated_abilities(&mut db).last().copied().unwrap(),
            mode: None,
        }]
    );

    Ok(())
}
