use std::collections::HashSet;

use pretty_assertions::assert_eq;

use crate::{
    battlefield::ActionResult,
    card::{Color, Keyword},
    effects::{Token, TokenCreature},
    in_play::{CardId, Database},
    load_cards,
    player::AllPlayers,
    stack::Stack,
    types::{Subtype, Type},
};

#[test]
fn creates_tokens() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();
    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let card = CardId::upload(&mut db, &cards, player, "Forbidden Friendship");
    card.move_to_stack(&mut db, vec![]);

    let results = Stack::resolve_1(&mut db);
    assert_eq!(
        results,
        [
            ActionResult::CreateToken {
                source: player.into(),
                token: Token::Creature(TokenCreature {
                    name: "Dinosaur".to_owned(),
                    types: HashSet::from([Type::Creature]),
                    subtypes: HashSet::from([Subtype::Dinosaur]),
                    colors: HashSet::from([Color::Red]),
                    keywords: HashSet::from([Keyword::Haste]),
                    power: 1,
                    toughness: 1
                })
            },
            ActionResult::CreateToken {
                source: player.into(),
                token: Token::Creature(TokenCreature {
                    name: "Human Soldier".to_owned(),
                    types: HashSet::from([Type::Creature]),
                    subtypes: HashSet::from([Subtype::Human, Subtype::Soldier]),
                    colors: HashSet::from([Color::White]),
                    keywords: HashSet::default(),
                    power: 1,
                    toughness: 1,
                })
            },
            ActionResult::StackToGraveyard(card),
        ]
    );

    Ok(())
}
