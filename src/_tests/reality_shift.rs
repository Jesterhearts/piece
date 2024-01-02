use indexmap::IndexSet;
use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, ResolutionResult},
    in_play::Database,
    in_play::{cards, CardId, InExile},
    load_cards,
    player::AllPlayers,
    stack::{ActiveTarget, Stack},
    turns::Turn,
    types::Subtype,
};

#[test]
fn resolves_shift() -> anyhow::Result<()> {
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

    let all_cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();
    let turn = Turn::new(&mut db, &all_players);

    let bear1 = CardId::upload(&mut db, &all_cards, player, "Alpine Grizzly");
    let bear2 = CardId::upload(&mut db, &all_cards, player, "Alpine Grizzly");
    let bear3 = CardId::upload(&mut db, &all_cards, player, "Alpine Grizzly");

    let mut results = Battlefield::add_from_stack_or_hand(&mut db, bear1, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, bear2, None);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    all_players[player].deck.place_on_top(&mut db, bear3);

    let shift = CardId::upload(&mut db, &all_cards, player, "Reality Shift");
    shift.move_to_stack(
        &mut db,
        vec![vec![ActiveTarget::Battlefield { id: bear1 }]],
        None,
        vec![],
    );

    let mut results = Stack::resolve_1(&mut db);
    let result = results.resolve(&mut db, &mut all_players, &turn, None);
    assert_eq!(result, ResolutionResult::Complete);

    assert_eq!(cards::<InExile>(&mut db), [bear1]);

    assert_eq!(bear2.power(&mut db), Some(4));
    assert_eq!(bear2.toughness(&mut db), Some(2));
    assert_eq!(bear2.subtypes(&db), IndexSet::from([Subtype::Bear]));

    assert_eq!(bear3.power(&mut db), Some(2));
    assert_eq!(bear3.toughness(&mut db), Some(2));
    assert_eq!(bear3.subtypes(&db), IndexSet::from([]));

    Ok(())
}
