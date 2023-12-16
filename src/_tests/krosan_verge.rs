use std::collections::VecDeque;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield, PendingResult, PendingResults, ResolutionResult},
    in_play::{CardId, Database, OnBattlefield},
    load_cards,
    mana::Mana,
    player::AllPlayers,
    stack::{ActiveTarget, Stack},
    turns::{Phase, Turn},
};

#[test]
fn enters_tapped() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);

    let card = CardId::upload(&mut db, &cards, player, "Krosan Verge");
    let results = Battlefield::add_from_stack_or_hand(&mut db, card);
    assert_eq!(results, PendingResults::default());

    assert!(card.tapped(&mut db));

    Ok(())
}

#[test]
fn tutors() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();
    let mut turn = Turn::new(&all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let forest = CardId::upload(&mut db, &cards, player, "Forest");
    all_players[player].deck.place_on_top(&mut db, forest);

    let plains = CardId::upload(&mut db, &cards, player, "Plains");
    all_players[player].deck.place_on_top(&mut db, plains);

    let annul = CardId::upload(&mut db, &cards, player, "Annul");
    all_players[player].deck.place_on_top(&mut db, annul);

    let card = CardId::upload(&mut db, &cards, player, "Krosan Verge");
    let results = Battlefield::add_from_stack_or_hand(&mut db, card);
    assert_eq!(results, PendingResults::default());

    card.untap(&mut db);

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, &turn, card, 0);
    let ability = *card.activated_abilities(&mut db).first().unwrap();
    assert_eq!(
        results,
        PendingResults {
            results: VecDeque::from([
                PendingResult {
                    apply_immediately: vec![],
                    to_resolve: Default::default(),
                    then_apply: vec![
                        ActionResult::TapPermanent(card),
                        ActionResult::PermanentToGraveyard(card),
                        ActionResult::SpendMana(player.into(), vec![Mana::Generic(2)]),
                    ],
                },
                PendingResult {
                    apply_immediately: vec![ActionResult::AddAbilityToStack {
                        source: card,
                        ability,
                        targets: vec![
                            vec![ActiveTarget::Library { id: forest }],
                            vec![ActiveTarget::Library { id: plains }],
                        ]
                    }],
                    to_resolve: Default::default(),
                    then_apply: vec![],
                },
            ]),
            applied: false,
        }
    );

    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert!(forest.is_in_location::<OnBattlefield>(&db));
    assert!(plains.is_in_location::<OnBattlefield>(&db));

    Ok(())
}
