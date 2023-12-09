use crate::{
    battlefield::Battlefield,
    in_play::{AbilityId, CardId},
    load_cards,
    player::AllPlayers,
    prepare_db,
    stack::Stack,
    types::Subtype,
};

#[test]
fn plains() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let card = CardId::upload(&db, &cards, player, "Plains")?;
    assert_eq!(
        card.activated_abilities(&db)?,
        [*AbilityId::land_abilities(&db)
            .get(&Subtype::Plains)
            .unwrap()]
    );

    Battlefield::add_from_stack(&db, card, vec![])?;
    let results = Battlefield::activate_ability(&db, &mut all_players, card, 0)?;
    Battlefield::maybe_resolve(&db, &mut all_players, results)?;

    let results = Stack::resolve_1(&db)?;
    Stack::apply_results(&db, &mut all_players, results)?;
    assert_eq!(all_players[player].mana_pool.white_mana, 1);

    Ok(())
}

#[test]
fn island() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let card = CardId::upload(&db, &cards, player, "Island")?;
    assert_eq!(
        card.activated_abilities(&db)?,
        [*AbilityId::land_abilities(&db)
            .get(&Subtype::Island)
            .unwrap()]
    );

    Battlefield::add_from_stack(&db, card, vec![])?;
    let results = Battlefield::activate_ability(&db, &mut all_players, card, 0)?;
    Battlefield::maybe_resolve(&db, &mut all_players, results)?;

    let results = Stack::resolve_1(&db)?;
    Stack::apply_results(&db, &mut all_players, results)?;
    assert_eq!(all_players[player].mana_pool.blue_mana, 1);

    Ok(())
}

#[test]
fn swamp() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let card = CardId::upload(&db, &cards, player, "Swamp")?;
    assert_eq!(
        card.activated_abilities(&db)?,
        [*AbilityId::land_abilities(&db).get(&Subtype::Swamp).unwrap()]
    );

    Battlefield::add_from_stack(&db, card, vec![])?;
    let results = Battlefield::activate_ability(&db, &mut all_players, card, 0)?;
    Battlefield::maybe_resolve(&db, &mut all_players, results)?;

    let results = Stack::resolve_1(&db)?;
    Stack::apply_results(&db, &mut all_players, results)?;
    assert_eq!(all_players[player].mana_pool.black_mana, 1);

    Ok(())
}

#[test]
fn mountain() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let card = CardId::upload(&db, &cards, player, "Mountain")?;
    assert_eq!(
        card.activated_abilities(&db)?,
        [*AbilityId::land_abilities(&db)
            .get(&Subtype::Mountain)
            .unwrap()]
    );

    Battlefield::add_from_stack(&db, card, vec![])?;
    let results = Battlefield::activate_ability(&db, &mut all_players, card, 0)?;
    Battlefield::maybe_resolve(&db, &mut all_players, results)?;

    let results = Stack::resolve_1(&db)?;
    Stack::apply_results(&db, &mut all_players, results)?;
    assert_eq!(all_players[player].mana_pool.red_mana, 1);

    Ok(())
}

#[test]
fn forest() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;
    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();

    let card = CardId::upload(&db, &cards, player, "Forest")?;
    assert_eq!(
        card.activated_abilities(&db)?,
        [*AbilityId::land_abilities(&db)
            .get(&Subtype::Forest)
            .unwrap()]
    );

    Battlefield::add_from_stack(&db, card, vec![])?;
    let results = Battlefield::activate_ability(&db, &mut all_players, card, 0)?;
    Battlefield::maybe_resolve(&db, &mut all_players, results)?;

    let results = Stack::resolve_1(&db)?;
    Stack::apply_results(&db, &mut all_players, results)?;
    assert_eq!(all_players[player].mana_pool.green_mana, 1);

    Ok(())
}
