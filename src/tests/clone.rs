use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield},
    deck::Deck,
    in_play::{AllCards, AllModifiers},
    load_cards,
    player::Player,
    stack::{Stack, StackResult},
};

#[test]
fn etb_clones() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let mut stack = Stack::default();
    let mut battlefield = Battlefield::default();
    let player = Player::new_ref(Deck::empty());

    let clone = all_cards.add(&cards, player.clone(), "Clone");
    let creature = all_cards.add(&cards, player.clone(), "Alpine Grizzly");
    let result = battlefield.add(&mut all_cards, &mut modifiers, creature);
    assert_eq!(result, []);

    stack.push_card(&all_cards, clone, None, None);

    let results = stack.resolve_1(&all_cards, &battlefield);
    assert_eq!(results, [StackResult::AddToBattlefield(clone)]);

    let [StackResult::AddToBattlefield(card)] = results.as_slice() else {
        unreachable!();
    };

    let mut results = battlefield.add(&mut all_cards, &mut modifiers, *card);
    assert_eq!(
        results,
        [ActionResult::CloneCreatureNonTargeting {
            source: clone,
            target: None
        }]
    );

    let [ActionResult::CloneCreatureNonTargeting { target, .. }] = results.as_mut_slice() else {
        unreachable!()
    };
    *target = Some(creature);

    battlefield.apply_action_results(&mut all_cards, &mut modifiers, &mut stack, results);

    let clone = &all_cards[clone].card;
    let creature = &all_cards[creature].card;
    assert_eq!(clone, creature);

    Ok(())
}

#[test]
fn etb_no_targets_dies() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut all_cards = AllCards::default();
    let mut modifiers = AllModifiers::default();
    let mut stack = Stack::default();
    let mut battlefield = Battlefield::default();
    let player = Player::new_ref(Deck::empty());

    let clone = all_cards.add(&cards, player.clone(), "Clone");

    stack.push_card(&all_cards, clone, None, None);

    let results = stack.resolve_1(&all_cards, &battlefield);
    assert_eq!(results, [StackResult::AddToBattlefield(clone)]);

    let [StackResult::AddToBattlefield(card)] = results.as_slice() else {
        unreachable!();
    };

    let results = battlefield.add(&mut all_cards, &mut modifiers, *card);
    assert_eq!(
        results,
        [ActionResult::CloneCreatureNonTargeting {
            source: clone,
            target: None
        }]
    );

    battlefield.apply_action_results(&mut all_cards, &mut modifiers, &mut stack, results);

    let results = battlefield.check_sba(&all_cards);

    assert_eq!(results, [ActionResult::PermanentToGraveyard(clone)]);

    Ok(())
}
