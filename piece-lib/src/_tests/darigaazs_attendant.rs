use itertools::Itertools;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    in_play::Database,
    load_cards,
    pending_results::ResolutionResult,
    player::AllPlayers,
    protogen::ids::CardId,
    protogen::{
        mana::ManaSource,
        mana::{Mana, ManaRestriction},
    },
    turns::Phase,
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

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);

    *all_players[player]
        .mana_pool
        .sourced
        .entry(Mana::COLORLESS)
        .or_default()
        .entry(ManaSource::ANY)
        .or_default()
        .entry(ManaRestriction::NONE)
        .or_default() = 1;
    let mut db = Database::new(all_players);

    db.turn.set_phase(Phase::PreCombatMainPhase);
    let attendant = CardId::upload(&mut db, &cards, player, "Darigaaz's Attendant");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, &attendant, None);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    let mut results = Battlefields::activate_ability(&mut db, &None, player, &attendant, 0);

    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(
        db.all_players[player].mana_pool.all_mana().collect_vec(),
        [
            (0, Mana::WHITE, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::BLUE, ManaSource::ANY, ManaRestriction::NONE),
            (1, Mana::BLACK, ManaSource::ANY, ManaRestriction::NONE),
            (1, Mana::RED, ManaSource::ANY, ManaRestriction::NONE),
            (1, Mana::GREEN, ManaSource::ANY, ManaRestriction::NONE),
            (0, Mana::COLORLESS, ManaSource::ANY, ManaRestriction::NONE)
        ]
    );

    Ok(())
}
