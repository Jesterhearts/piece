use std::{collections::VecDeque, vec};

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{ActionResult, Battlefield, PendingResult, PendingResults, ResolutionResult},
    in_play::{CardId, Database},
    load_cards,
    mana::Mana,
    player::AllPlayers,
    turns::{Phase, Turn},
};

#[test]
fn sacrifice_draw_card() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let mut turn = Turn::new(&all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let results = Battlefield::add_from_stack_or_hand(&mut db, card);
    assert_eq!(results, PendingResults::default());

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, &turn, card, 0);
    let ability = card.activated_abilities(&mut db).first().copied().unwrap();
    assert_eq!(
        results,
        PendingResults {
            results: VecDeque::from([
                PendingResult {
                    apply_immediately: vec![],
                    to_resolve: VecDeque::default(),
                    then_apply: vec![
                        ActionResult::TapPermanent(card),
                        ActionResult::PermanentToGraveyard(card),
                        ActionResult::SpendMana(
                            player.into(),
                            vec![Mana::White, Mana::Black, Mana::Green]
                        )
                    ],
                },
                PendingResult {
                    apply_immediately: vec![ActionResult::AddAbilityToStack {
                        source: card,
                        ability,
                        targets: vec![vec![]]
                    }],
                    to_resolve: Default::default(),
                    then_apply: vec![],
                }
            ]),
            applied: false,
        }
    );

    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::Complete);

    Ok(())
}

#[test]
fn add_mana() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    let mut turn = Turn::new(&all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let card = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let results = Battlefield::add_from_stack_or_hand(&mut db, card);
    assert_eq!(results, PendingResults::default());

    let mut results = Battlefield::activate_ability(&mut db, &mut all_players, &turn, card, 1);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(result, ResolutionResult::PendingChoice);

    let result = results.resolve(&mut db, &mut all_players, Some(0));
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(all_players[player].mana_pool.white_mana, 1);

    Ok(())
}
