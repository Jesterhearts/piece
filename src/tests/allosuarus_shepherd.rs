use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::{Entry, Stack, StackResult},
    types::Subtype,
};

#[test]
fn modify_base_p_t_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let card = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    let results = Battlefield::add_from_stack(&mut db, card, vec![]);
    assert_eq!(results, []);

    let results = Battlefield::activate_ability(&mut db, &mut all_players, card, 0);

    assert_eq!(
        results,
        [UnresolvedActionResult::AddAbilityToStack {
            source: card,
            ability: card.activated_abilities(&mut db).first().copied().unwrap(),
            valid_targets: Default::default(),
        }]
    );

    let results = Battlefield::maybe_resolve(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    let results = Stack::resolve_1(&mut db);
    assert!(matches!(
        results.as_slice(),
        [StackResult::ApplyToBattlefield(_),]
    ));

    let results = Stack::apply_results(&mut db, &mut all_players, results);
    assert_eq!(results, []);

    assert_eq!(card.power(&mut db), Some(5));
    assert_eq!(card.toughness(&mut db), Some(5));
    assert_eq!(
        card.subtypes(&mut db),
        HashSet::from([Subtype::Elf, Subtype::Shaman, Subtype::Dinosaur])
    );

    Battlefield::end_turn(&mut db);

    assert_eq!(card.power(&mut db), Some(1));
    assert_eq!(card.toughness(&mut db), Some(1));
    assert_eq!(
        card.subtypes(&mut db),
        HashSet::from([Subtype::Elf, Subtype::Shaman])
    );

    Ok(())
}

#[test]
fn does_not_resolve_counterspells_respecting_uncounterable() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let card = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    let counterspell = CardId::upload(&mut db, &cards, player, "Counterspell");

    card.move_to_stack(&mut db, HashSet::default());
    let targets = HashSet::from([Stack::target_nth(&mut db, 0)]);
    counterspell.move_to_stack(&mut db, targets);

    assert_eq!(Stack::in_stack(&mut db).len(), 2);

    let results = Stack::resolve_1(&mut db);
    assert_eq!(results, [StackResult::StackToGraveyard(counterspell)]);
    Stack::apply_results(&mut db, &mut all_players, results);

    assert_eq!(Stack::in_stack(&mut db).len(), 1);

    let results = Stack::resolve_1(&mut db);
    assert_eq!(results, [StackResult::AddToBattlefield(card)]);

    Ok(())
}

#[test]
fn does_not_resolve_counterspells_respecting_green_uncounterable() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let card1 = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    let card2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let counterspell = CardId::upload(&mut db, &cards, player, "Counterspell");

    let results = Battlefield::add_from_stack(&mut db, card1, vec![]);
    assert_eq!(results, []);

    card2.move_to_stack(&mut db, HashSet::default());
    let targets = HashSet::from([Stack::target_nth(&mut db, 0)]);
    counterspell.move_to_stack(&mut db, targets);

    assert_eq!(Stack::in_stack(&mut db).len(), 2);

    let results = Stack::resolve_1(&mut db);
    assert_eq!(results, [StackResult::StackToGraveyard(counterspell)]);
    Stack::apply_results(&mut db, &mut all_players, results);

    assert_eq!(Stack::in_stack(&mut db).len(), 1);

    let results = Stack::resolve_1(&mut db);
    assert_eq!(results, [StackResult::AddToBattlefield(card2)]);

    Ok(())
}

#[test]
fn resolves_counterspells_respecting_green_uncounterable_other_player() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    let player2 = all_players.new_player();
    all_players[player].infinite_mana();

    let card1 = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    let card2 = CardId::upload(&mut db, &cards, player2, "Alpine Grizzly");
    let counterspell = CardId::upload(&mut db, &cards, player, "Counterspell");

    let results = Battlefield::add_from_stack(&mut db, card1, vec![]);
    assert_eq!(results, []);

    card2.move_to_stack(&mut db, HashSet::default());
    let targets = HashSet::from([Stack::target_nth(&mut db, 0)]);
    counterspell.move_to_stack(&mut db, targets);

    assert_eq!(Stack::in_stack(&mut db).len(), 2);

    let results = Stack::resolve_1(&mut db);
    assert_eq!(
        results,
        [
            StackResult::SpellCountered {
                id: Entry::Card(card2)
            },
            StackResult::StackToGraveyard(counterspell)
        ]
    );
    Stack::apply_results(&mut db, &mut all_players, results);

    assert!(Stack::is_empty(&mut db));

    Ok(())
}
