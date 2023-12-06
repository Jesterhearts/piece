use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefield,
    deck::Deck,
    in_play::{AllCards, AllModifiers},
    load_cards,
    player::Player,
    stack::{Stack, StackResult},
};

#[test]
fn resolves_counterspells() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let player = Player::new_ref(Deck::empty());
    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let mut battlefield = Battlefield::default();
    let mut stack = Stack::default();

    let counterspell_1 = all_cards.add(&cards, player.clone(), "Counterspell");
    let counterspell_2 = all_cards.add(&cards, player.clone(), "Counterspell");

    let countered = stack.push_card(&all_cards, counterspell_1, None, None);

    stack.push_card(&all_cards, counterspell_2, stack.target_nth(0), None);

    assert_eq!(stack.stack.len(), 2);

    let results = stack.resolve_1(&all_cards, &battlefield);
    assert_eq!(results, [StackResult::SpellCountered { id: countered }]);
    let results = stack.apply_results(&mut all_cards, &mut modifiers, &mut battlefield, results);
    assert_eq!(results, []);

    assert!(stack.is_empty());

    Ok(())
}
