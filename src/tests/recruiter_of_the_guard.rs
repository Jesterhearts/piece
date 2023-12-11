use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::{Battlefield, UnresolvedActionResult},
    effects::Destination,
    in_play::CardId,
    in_play::Database,
    load_cards,
    player::AllPlayers,
    targets::{Comparison, Restriction},
    types::Type,
};

#[test]
fn etb() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let bear = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    all_players[player].deck.place_on_top(&mut db, bear);

    let spell = CardId::upload(&mut db, &cards, player, "Annul");
    all_players[player].deck.place_on_top(&mut db, spell);

    let elesh = CardId::upload(&mut db, &cards, player, "Elesh Norn, Grand Cenobite");
    all_players[player].deck.place_on_top(&mut db, elesh);

    let recruiter = CardId::upload(&mut db, &cards, player, "Recruiter of the Guard");
    recruiter.move_to_hand(&mut db);
    let results = Battlefield::add_from_stack(&mut db, recruiter, vec![]);
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
