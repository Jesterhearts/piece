use pretty_assertions::assert_eq;

use crate::{
    battlefield::Battlefields,
    effects::{EffectBehaviors, PendingEffects, SelectedStack, SelectionResult},
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    protogen::{
        effects::{MoveToBattlefield, MoveToGraveyard},
        targets::Location,
    },
    stack::{Selected, TargetType},
};

#[test]
fn aura_works() -> anyhow::Result<()> {
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

    let mut db = Database::new(all_players);
    let mut results = PendingEffects::default();

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    results.apply_results(MoveToBattlefield::default().apply(
        &mut db,
        None,
        &mut SelectedStack::new(vec![Selected {
            location: Some(Location::ON_BATTLEFIELD),
            target_type: TargetType::Card(creature),
            targeted: false,
            restrictions: vec![],
        }]),
        false,
    ));

    let aura = CardId::upload(&mut db, &cards, player, "Abzan Runemark");
    let mut selected = SelectedStack::new(vec![Selected {
        location: Some(Location::ON_BATTLEFIELD),
        target_type: TargetType::Card(creature),
        targeted: true,
        restrictions: vec![],
    }]);
    selected.save();
    selected.clear();
    selected.push(Selected {
        location: Some(Location::IN_STACK),
        target_type: TargetType::Card(aura),
        targeted: false,
        restrictions: vec![],
    });
    results.apply_results(MoveToBattlefield::default().apply(&mut db, None, &mut selected, false));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(creature.power(&db), Some(6));
    assert_eq!(creature.toughness(&db), Some(4));
    assert!(creature.vigilance(&db));

    let card2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    card2.move_to_battlefield(&mut db);

    assert_eq!(card2.power(&db), Some(4));
    assert_eq!(card2.toughness(&db), Some(2));

    results.apply_results(MoveToGraveyard::default().apply(
        &mut db,
        None,
        &mut SelectedStack::new(vec![Selected {
            location: Some(Location::ON_BATTLEFIELD),
            target_type: TargetType::Card(aura),
            targeted: false,
            restrictions: vec![],
        }]),
        false,
    ));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(creature.power(&db), Some(4));
    assert_eq!(creature.toughness(&db), Some(2));
    assert!(!creature.vigilance(&db));

    assert!(Battlefields::no_modifiers(&db));

    Ok(())
}

#[test]
fn aura_leaves_battlefield_enchanting_leaves_battlefield() -> anyhow::Result<()> {
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

    let mut db = Database::new(all_players);
    let mut results = PendingEffects::default();

    let creature = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    results.apply_results(MoveToBattlefield::default().apply(
        &mut db,
        None,
        &mut SelectedStack::new(vec![Selected {
            location: Some(Location::ON_BATTLEFIELD),
            target_type: TargetType::Card(creature),
            targeted: false,
            restrictions: vec![],
        }]),
        false,
    ));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let aura = CardId::upload(&mut db, &cards, player, "Abzan Runemark");
    let mut selected = SelectedStack::new(vec![Selected {
        location: Some(Location::ON_BATTLEFIELD),
        target_type: TargetType::Card(creature),
        targeted: true,
        restrictions: vec![],
    }]);
    selected.save();
    selected.clear();
    selected.push(Selected {
        location: Some(Location::IN_STACK),
        target_type: TargetType::Card(aura),
        targeted: false,
        restrictions: vec![],
    });
    results.apply_results(MoveToBattlefield::default().apply(&mut db, None, &mut selected, false));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert_eq!(creature.power(&db), Some(6));
    assert_eq!(creature.toughness(&db), Some(4));
    assert!(creature.vigilance(&db));

    let mut results = Battlefields::check_sba(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = PendingEffects::default();
    results.apply_results(MoveToGraveyard::default().apply(
        &mut db,
        None,
        &mut SelectedStack::new(vec![Selected {
            location: Some(Location::ON_BATTLEFIELD),
            target_type: TargetType::Card(creature),
            targeted: false,
            restrictions: vec![],
        }]),
        false,
    ));
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    let mut results = Battlefields::check_sba(&mut db);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::TryAgain);
    let result = results.resolve(&mut db, None);
    assert_eq!(result, SelectionResult::Complete);

    assert!(db.battlefield.is_empty());
    assert!(Battlefields::no_modifiers(&db));

    Ok(())
}

#[test]
fn vigilance_is_lost_no_green_permanent() -> anyhow::Result<()> {
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

    let mut db = Database::new(all_players);
    let mut results = PendingEffects::default();

    let creature = CardId::upload(&mut db, &cards, player, "Recruiter of the Guard");
    results.apply_results(MoveToBattlefield::default().apply(
        &mut db,
        None,
        &mut SelectedStack::new(vec![Selected {
            location: Some(Location::ON_BATTLEFIELD),
            target_type: TargetType::Card(creature),
            targeted: false,
            restrictions: vec![],
        }]),
        false,
    ));

    let aura = CardId::upload(&mut db, &cards, player, "Abzan Runemark");
    let mut selected = SelectedStack::new(vec![Selected {
        location: Some(Location::ON_BATTLEFIELD),
        target_type: TargetType::Card(creature),
        targeted: true,
        restrictions: vec![],
    }]);
    selected.save();
    selected.push(Selected {
        location: Some(Location::IN_STACK),
        target_type: TargetType::Card(aura),
        targeted: false,
        restrictions: vec![],
    });
    results.apply_results(MoveToBattlefield::default().apply(&mut db, None, &mut selected, false));

    assert_eq!(creature.power(&db), Some(3));
    assert_eq!(creature.toughness(&db), Some(3));
    assert!(!creature.vigilance(&db));

    let card2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    results.apply_results(MoveToBattlefield::default().apply(
        &mut db,
        None,
        &mut SelectedStack::new(vec![Selected {
            location: Some(Location::ON_BATTLEFIELD),
            target_type: TargetType::Card(card2),
            targeted: false,
            restrictions: vec![],
        }]),
        false,
    ));

    assert_eq!(card2.power(&db), Some(4));
    assert_eq!(card2.toughness(&db), Some(2));
    assert!(creature.vigilance(&db));

    let mut results = PendingEffects::default();
    results.apply_results(MoveToGraveyard::default().apply(
        &mut db,
        None,
        &mut SelectedStack::new(vec![Selected {
            location: Some(Location::ON_BATTLEFIELD),
            target_type: TargetType::Card(card2),
            targeted: false,
            restrictions: vec![],
        }]),
        false,
    ));
    assert!(results.is_empty());
    assert!(!creature.vigilance(&db));

    Ok(())
}
