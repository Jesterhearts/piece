use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield, UnresolvedActionResult},
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
    let result = battlefield.add(&mut all_cards, &mut modifiers, creature, vec![]);
    assert_eq!(result, []);

    stack.push_card(&all_cards, clone, None, None);

    let results = stack.resolve_1(&all_cards, &battlefield);
    assert_eq!(results, [StackResult::AddToBattlefield(clone)]);

    let [StackResult::AddToBattlefield(card)] = results.as_slice() else {
        unreachable!();
    };

    let results = battlefield.add(&mut all_cards, &mut modifiers, *card, vec![]);
    assert_eq!(
        results,
        [UnresolvedActionResult::CloneCreatureNonTargeting {
            source: clone,
            valid_targets: vec![creature]
        }]
    );

    let results = results
        .into_iter()
        .map(|result| match result {
            UnresolvedActionResult::CloneCreatureNonTargeting {
                source,
                mut valid_targets,
            } => ActionResult::CloneCreatureNonTargeting {
                source,
                target: valid_targets.pop(),
            },
            _ => unreachable!(),
        })
        .collect();

    let results =
        battlefield.apply_action_results(&mut all_cards, &mut modifiers, &mut stack, results);
    assert_eq!(results, []);

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

    let results = battlefield.add(&mut all_cards, &mut modifiers, *card, vec![]);
    assert_eq!(
        results,
        [UnresolvedActionResult::CloneCreatureNonTargeting {
            source: clone,
            valid_targets: vec![],
        }]
    );

    let results = results
        .into_iter()
        .map(|result| match result {
            UnresolvedActionResult::CloneCreatureNonTargeting {
                source,
                mut valid_targets,
            } => ActionResult::CloneCreatureNonTargeting {
                source,
                target: valid_targets.pop(),
            },
            _ => unreachable!(),
        })
        .collect();

    let results =
        battlefield.apply_action_results(&mut all_cards, &mut modifiers, &mut stack, results);
    assert_eq!(results, []);

    let results = battlefield.check_sba(&all_cards);

    assert_eq!(results, [ActionResult::PermanentToGraveyard(clone)]);

    Ok(())
}
