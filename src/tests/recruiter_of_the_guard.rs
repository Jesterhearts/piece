use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    effects::Destination,
    in_play::CardId,
    load_cards,
    player::AllPlayers,
    prepare_db,
    targets::{Comparison, Restriction},
    types::Type,
};

#[test]
fn etb() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player();
    all_players[player].infinite_mana();

    let bear = CardId::upload(&db, &cards, player, "Alpine Grizzly")?;
    all_players[player].deck.place_on_top(&db, bear)?;

    let spell = CardId::upload(&db, &cards, player, "Annul")?;
    all_players[player].deck.place_on_top(&db, spell)?;

    let elesh = CardId::upload(&db, &cards, player, "Elesh Norn, Grand Cenobite")?;
    all_players[player].deck.place_on_top(&db, elesh)?;

    let recruiter = CardId::upload(&db, &cards, player, "Recruiter of the Guard")?;
    recruiter.move_to_hand(&db)?;
    let results = Battlefield::add_from_stack(&db, recruiter, vec![])?;
    assert_eq!(
        results,
        [UnresolvedActionResult::TutorLibrary {
            source: recruiter,
            destination: Destination::Hand,
            targets: vec![bear],
            reveal: true,
            restrictions: vec![
                Restriction::OfType {
                    types: HashSet::from([Type::Creature]),
                    subtypes: Default::default(),
                },
                Restriction::Toughness(Comparison::LessThanOrEqual(2)),
            ]
        }]
    );

    Ok(())
}
