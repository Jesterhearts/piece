use crate::{
    in_play::{AbilityId, CardId},
    load_cards,
    player::AllPlayers,
    prepare_db,
    types::Subtype,
};

#[test]
fn plains() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;
    let player = AllPlayers::default().new_player();

    let card = CardId::upload(&db, &cards, player, "Plains")?;
    assert_eq!(
        card.activated_abilities(&db)?,
        [*AbilityId::land_abilities(&db)
            .get(&Subtype::Plains)
            .unwrap()]
    );

    Ok(())
}

#[test]
fn island() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;
    let player = AllPlayers::default().new_player();

    let card = CardId::upload(&db, &cards, player, "Island")?;
    assert_eq!(
        card.activated_abilities(&db)?,
        [*AbilityId::land_abilities(&db)
            .get(&Subtype::Island)
            .unwrap()]
    );

    Ok(())
}

#[test]
fn swamp() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;
    let player = AllPlayers::default().new_player();

    let card = CardId::upload(&db, &cards, player, "Swamp")?;
    assert_eq!(
        card.activated_abilities(&db)?,
        [*AbilityId::land_abilities(&db).get(&Subtype::Swamp).unwrap()]
    );

    Ok(())
}

#[test]
fn mountain() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;
    let player = AllPlayers::default().new_player();

    let card = CardId::upload(&db, &cards, player, "Mountain")?;
    assert_eq!(
        card.activated_abilities(&db)?,
        [*AbilityId::land_abilities(&db)
            .get(&Subtype::Mountain)
            .unwrap()]
    );

    Ok(())
}

#[test]
fn forest() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;
    let player = AllPlayers::default().new_player();

    let card = CardId::upload(&db, &cards, player, "Forest")?;
    assert_eq!(
        card.activated_abilities(&db)?,
        [*AbilityId::land_abilities(&db)
            .get(&Subtype::Forest)
            .unwrap()]
    );

    Ok(())
}
