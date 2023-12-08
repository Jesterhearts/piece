use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    in_play::{CardId, ModifierId},
    load_cards,
    player::AllPlayers,
    prepare_db,
    stack::{Entry, Stack, StackResult},
    types::Subtype,
};

#[test]
fn modify_base_p_t_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let card = CardId::upload(&db, &cards, player, "Allosaurus Shepherd")?;
    let results = Battlefield::add(&db, card, vec![])?;
    assert_eq!(results, []);

    let results = Battlefield::activate_ability(&db, &mut all_players, card, 0)?;

    assert_eq!(
        results,
        [UnresolvedActionResult::AddAbilityToStack {
            ability: card
                .activated_abilities(&db)?
                .first()
                .copied()
                .unwrap_or_default(),
            valid_targets: Default::default(),
        }]
    );

    let results = Battlefield::maybe_resolve(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    let results = Stack::resolve_1(&db)?;
    assert_eq!(
        results,
        [StackResult::ApplyToBattlefield(ModifierId::default()),]
    );

    let results = Stack::apply_results(&db, &mut all_players, results)?;
    assert_eq!(results, []);

    assert_eq!(card.power(&db)?, Some(5));
    assert_eq!(card.toughness(&db)?, Some(5));
    assert_eq!(
        card.subtypes(&db)?,
        HashSet::from([Subtype::Elf, Subtype::Shaman, Subtype::Dinosaur])
    );

    Battlefield::end_turn(&db)?;

    assert_eq!(card.power(&db)?, Some(1));
    assert_eq!(card.toughness(&db)?, Some(1));
    assert_eq!(
        card.subtypes(&db)?,
        HashSet::from([Subtype::Elf, Subtype::Shaman])
    );

    Ok(())
}

#[test]
fn does_not_resolve_counterspells_respecting_uncounterable() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let card = CardId::upload(&db, &cards, player, "Allosaurus Shepherd")?;
    let counterspell = CardId::upload(&db, &cards, player, "Counterspell")?;

    card.move_to_stack(&db, HashSet::default())?;
    counterspell.move_to_stack(&db, HashSet::from([Stack::target_nth(&db, 0)?]))?;

    assert_eq!(Stack::in_stack(&db)?.len(), 2);

    let results = Stack::resolve_1(&db)?;
    assert_eq!(results, [StackResult::StackToGraveyard(counterspell)]);
    Stack::apply_results(&db, &mut all_players, results)?;

    assert_eq!(Stack::in_stack(&db)?.len(), 1);

    let results = Stack::resolve_1(&db)?;
    assert_eq!(results, [StackResult::AddToBattlefield(card)]);

    Ok(())
}

#[test]
fn does_not_resolve_counterspells_respecting_green_uncounterable() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let card1 = CardId::upload(&db, &cards, player, "Allosaurus Shepherd")?;
    let card2 = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
    let counterspell = CardId::upload(&db, &cards, player, "Counterspell")?;

    Battlefield::add(&db, card1, vec![])?;

    card2.move_to_stack(&db, HashSet::default())?;
    counterspell.move_to_stack(&db, HashSet::from([Stack::target_nth(&db, 0)?]))?;

    assert_eq!(Stack::in_stack(&db)?.len(), 2);

    let results = Stack::resolve_1(&db)?;
    assert_eq!(results, [StackResult::StackToGraveyard(counterspell)]);
    Stack::apply_results(&db, &mut all_players, results)?;

    assert_eq!(Stack::in_stack(&db)?.len(), 1);

    let results = Stack::resolve_1(&db)?;
    assert_eq!(results, [StackResult::AddToBattlefield(card2)]);

    Ok(())
}

#[test]
fn resolves_counterspells_respecting_green_uncounterable_other_player() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    let player2 = all_players.new_player();
    all_players[player].infinite_mana();

    let card1 = CardId::upload(&db, &cards, player, "Allosaurus Shepherd")?;
    let card2 = CardId::upload(&db, &cards, player2, "Alpine Grizzly")?;
    let counterspell = CardId::upload(&db, &cards, player, "Counterspell")?;

    Battlefield::add(&db, card1, vec![])?;

    card2.move_to_stack(&db, HashSet::default())?;
    counterspell.move_to_stack(&db, HashSet::from([Stack::target_nth(&db, 0)?]))?;

    assert_eq!(Stack::in_stack(&db)?.len(), 2);

    let results = Stack::resolve_1(&db)?;
    assert_eq!(
        results,
        [
            StackResult::SpellCountered {
                id: Entry::Card(card2)
            },
            StackResult::StackToGraveyard(counterspell)
        ]
    );
    Stack::apply_results(&db, &mut all_players, results)?;

    assert!(Stack::is_empty(&db)?);

    Ok(())
}
