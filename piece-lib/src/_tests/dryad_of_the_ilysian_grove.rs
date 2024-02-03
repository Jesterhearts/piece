use pretty_assertions::assert_eq;

use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::{AllPlayers, Player},
    protogen::{effects::MoveToBattlefield, targets::Location, types::Subtype},
    stack::{Selected, TargetType},
    types::SubtypeSet,
};

#[test]
fn adds_land_types() -> anyhow::Result<()> {
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
    all_players[player].infinite_mana();

    let mut db = Database::new(all_players);

    let land = CardId::upload(&mut db, &cards, player, "Forest");
    land.move_to_battlefield(&mut db);

    let card = CardId::upload(&mut db, &cards, player, "Dryad of the Ilysian Grove");
    let mut results = PendingEffects::default();
    results.selected.push(Selected {
        location: Some(Location::ON_BATTLEFIELD),
        target_type: TargetType::Card(card),
        targeted: false,
        restrictions: vec![],
    });
    let to_apply = MoveToBattlefield::default().apply(&mut db, None, &mut results.selected, false);
    results.apply_results(to_apply);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(Player::lands_per_turn(&mut db, player), 2);

    assert_eq!(
        db[land].modified_subtypes,
        SubtypeSet::from([
            Subtype::PLAINS,
            Subtype::ISLAND,
            Subtype::SWAMP,
            Subtype::MOUNTAIN,
            Subtype::FOREST,
        ])
    );

    Ok(())
}
