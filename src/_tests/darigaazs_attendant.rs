use itertools::Itertools;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    mana::{Mana, ManaRestriction},
    player::{mana_pool::ManaSource, AllPlayers},
    turns::{Phase, Turn},
};

#[test]
fn sacrifice_gain_mana() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .pretty()
        .with_ansi(false)
        .with_line_number(true)
        .with_file(true)
        .with_target(false)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ENTER)
        .with_writer(std::io::stderr)
        .try_init();

    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    *all_players[player]
        .mana_pool
        .sourced
        .entry(Mana::Colorless)
        .or_default()
        .entry(ManaSource::Any)
        .or_default()
        .entry(ManaRestriction::None)
        .or_default() = 1;

    let mut turn = Turn::new(&mut db, &all_players);
    turn.set_phase(Phase::PreCombatMainPhase);

    let attendant = CardId::upload(&mut db, &cards, player, "Darigaaz's Attendant");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, attendant, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results =
        Battlefield::activate_ability(&mut db, &mut all_players, &turn, player, attendant, 0);

    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(
        all_players[player].mana_pool.all_mana().collect_vec(),
        [
            (0, Mana::White, ManaSource::Any, ManaRestriction::None),
            (0, Mana::Blue, ManaSource::Any, ManaRestriction::None),
            (1, Mana::Black, ManaSource::Any, ManaRestriction::None),
            (1, Mana::Red, ManaSource::Any, ManaRestriction::None),
            (1, Mana::Green, ManaSource::Any, ManaRestriction::None),
            (0, Mana::Colorless, ManaSource::Any, ManaRestriction::None)
        ]
    );

    Ok(())
}
