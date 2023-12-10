use crate::{
    battlefield::Battlefield,
    in_play::Database,
    in_play::{AbilityId, CardId},
    load_cards,
    player::AllPlayers,
    stack::Stack,
    types::Subtype,
};

#[test]
fn plains() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let card = CardId::upload(&mut db, &cards, player, "Plains");
    assert_eq!(
        card.activated_abilities(&mut db),
        [*AbilityId::land_abilities(&mut db)
            .get(&Subtype::Plains)
            .unwrap()]
    );

    Battlefield::add_from_stack(&mut db, card, vec![]);
    let results = Battlefield::activate_ability(&mut db, &mut all_players, card, 0);
    Battlefield::maybe_resolve(&mut db, &mut all_players, results);

    let results = Stack::resolve_1(&mut db);
    Stack::apply_results(&mut db, &mut all_players, results);
    assert_eq!(all_players[player].mana_pool.white_mana, 1);

    Ok(())
}

#[test]
fn island() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let card = CardId::upload(&mut db, &cards, player, "Island");
    assert_eq!(
        card.activated_abilities(&mut db),
        [*AbilityId::land_abilities(&mut db)
            .get(&Subtype::Island)
            .unwrap()]
    );

    Battlefield::add_from_stack(&mut db, card, vec![]);
    let results = Battlefield::activate_ability(&mut db, &mut all_players, card, 0);
    Battlefield::maybe_resolve(&mut db, &mut all_players, results);

    let results = Stack::resolve_1(&mut db);
    Stack::apply_results(&mut db, &mut all_players, results);
    assert_eq!(all_players[player].mana_pool.blue_mana, 1);

    Ok(())
}

#[test]
fn swamp() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let card = CardId::upload(&mut db, &cards, player, "Swamp");
    assert_eq!(
        card.activated_abilities(&mut db),
        [*AbilityId::land_abilities(&mut db)
            .get(&Subtype::Swamp)
            .unwrap()]
    );

    Battlefield::add_from_stack(&mut db, card, vec![]);
    let results = Battlefield::activate_ability(&mut db, &mut all_players, card, 0);
    Battlefield::maybe_resolve(&mut db, &mut all_players, results);

    let results = Stack::resolve_1(&mut db);
    Stack::apply_results(&mut db, &mut all_players, results);
    assert_eq!(all_players[player].mana_pool.black_mana, 1);

    Ok(())
}

#[test]
fn mountain() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let card = CardId::upload(&mut db, &cards, player, "Mountain");
    assert_eq!(
        card.activated_abilities(&mut db),
        [*AbilityId::land_abilities(&mut db)
            .get(&Subtype::Mountain)
            .unwrap()]
    );

    Battlefield::add_from_stack(&mut db, card, vec![]);
    let results = Battlefield::activate_ability(&mut db, &mut all_players, card, 0);
    Battlefield::maybe_resolve(&mut db, &mut all_players, results);

    let results = Stack::resolve_1(&mut db);
    Stack::apply_results(&mut db, &mut all_players, results);
    assert_eq!(all_players[player].mana_pool.red_mana, 1);

    Ok(())
}

#[test]
fn forest() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let card = CardId::upload(&mut db, &cards, player, "Forest");
    assert_eq!(
        card.activated_abilities(&mut db),
        [*AbilityId::land_abilities(&mut db)
            .get(&Subtype::Forest)
            .unwrap()]
    );

    Battlefield::add_from_stack(&mut db, card, vec![]);
    let results = Battlefield::activate_ability(&mut db, &mut all_players, card, 0);
    Battlefield::maybe_resolve(&mut db, &mut all_players, results);

    let results = Stack::resolve_1(&mut db);
    Stack::apply_results(&mut db, &mut all_players, results);
    assert_eq!(all_players[player].mana_pool.green_mana, 1);

    Ok(())
}
