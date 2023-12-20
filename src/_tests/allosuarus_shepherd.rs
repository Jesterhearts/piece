use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield, ResolutionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    stack::{Entry, Stack},
    turns::{Phase, Turn},
    types::Subtype,
};

#[test]
fn modify_base_p_t_works() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let mut turn = Turn::new(&all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card, None);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, &turn, card, 0);

    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // Pay costs
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::PendingChoice);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    // end pay costs
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(card.power(&db), Some(5));
    assert_eq!(card.toughness(&db), Some(5));
    assert_eq!(
        card.subtypes(&db),
        IndexSet::from([Subtype::Elf, Subtype::Shaman, Subtype::Dinosaur])
    );

    Battlefield::end_turn(&mut db);

    assert_eq!(card.power(&db), Some(1));
    assert_eq!(card.toughness(&db), Some(1));
    assert_eq!(
        card.subtypes(&db),
        IndexSet::from([Subtype::Elf, Subtype::Shaman])
    );

    Ok(())
}

#[test]
fn does_not_resolve_counterspells_respecting_uncounterable() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let card = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    let counterspell = CardId::upload(&mut db, &cards, player, "Counterspell");

    card.move_to_stack(&mut db, vec![], None);
    let targets = vec![vec![Stack::target_nth(&mut db, 0)]];
    counterspell.move_to_stack(&mut db, targets, None);

    assert_eq!(Stack::in_stack(&mut db).len(), 2);

    let mut results = Stack::resolve_1(&mut db);
    assert_eq!(
        results,
        [ActionResult::StackToGraveyard(counterspell)].into()
    );
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(Stack::in_stack(&mut db).len(), 1);

    let results = Stack::resolve_1(&mut db);
    assert_eq!(
        results,
        (card, true, [ActionResult::AddToBattlefield(card, None)]).into()
    );

    Ok(())
}

#[test]
fn does_not_resolve_counterspells_respecting_green_uncounterable() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let card1 = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    let card2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let counterspell = CardId::upload(&mut db, &cards, player, "Counterspell");

    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card1, None);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);
    card2.move_to_stack(&mut db, vec![], None);
    let targets = vec![vec![Stack::target_nth(&mut db, 0)]];
    counterspell.move_to_stack(&mut db, targets, None);

    assert_eq!(Stack::in_stack(&mut db).len(), 2);

    let mut results = Stack::resolve_1(&mut db);
    assert_eq!(
        results,
        [ActionResult::StackToGraveyard(counterspell)].into()
    );
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(Stack::in_stack(&mut db).len(), 1);

    let results = Stack::resolve_1(&mut db);
    assert_eq!(
        results,
        (card2, true, [ActionResult::AddToBattlefield(card2, None)]).into()
    );

    Ok(())
}

#[test]
fn resolves_counterspells_respecting_green_uncounterable_other_player() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    let player2 = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let card1 = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    let card2 = CardId::upload(&mut db, &cards, player2, "Alpine Grizzly");
    let counterspell = CardId::upload(&mut db, &cards, player, "Counterspell");

    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card1, None);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);
    card2.move_to_stack(&mut db, vec![], None);
    let targets = vec![vec![Stack::target_nth(&mut db, 0)]];
    counterspell.move_to_stack(&mut db, targets, None);

    assert_eq!(Stack::in_stack(&mut db).len(), 2);

    let mut results = Stack::resolve_1(&mut db);
    assert_eq!(
        results,
        (
            counterspell,
            true,
            [
                ActionResult::SpellCountered {
                    id: Entry::Card(card2)
                },
                ActionResult::StackToGraveyard(counterspell)
            ]
        )
            .into()
    );
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert!(Stack::is_empty(&mut db));

    Ok(())
}
