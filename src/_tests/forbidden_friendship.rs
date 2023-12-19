use std::collections::HashSet;

use indexmap::IndexSet;
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
    let player = all_players.new_player("Player".to_string(), 20);
    all_players[player].infinite_mana();

    let card = CardId::upload(&mut db, &cards, player, "Forbidden Friendship");
    let targets = card.valid_targets(&mut db);
    card.move_to_stack(&mut db, targets, None);

    let results = Stack::resolve_1(&mut db);
    assert_eq!(
        results,
        (
            card,
            true,
            [
                ActionResult::CreateToken {
                    source: player.into(),
                    token: Box::new(Token::Creature(TokenCreature {
                        name: "Dinosaur".to_string(),
                        types: IndexSet::from([Type::Creature]),
                        subtypes: IndexSet::from([Subtype::Dinosaur]),
                        colors: HashSet::from([Color::Red]),
                        keywords: [Keyword::Haste].into_iter().collect(),
                        power: 1,
                        toughness: 1
                    }))
                },
                ActionResult::CreateToken {
                    source: player.into(),
                    token: Box::new(Token::Creature(TokenCreature {
                        name: "Human Soldier".to_string(),
                        types: IndexSet::from([Type::Creature]),
                        subtypes: IndexSet::from([Subtype::Human, Subtype::Soldier]),
                        colors: HashSet::from([Color::White]),
                        keywords: ::counter::Counter::default(),
                        power: 1,
                        toughness: 1,
                    }))
                },
                ActionResult::StackToGraveyard(card),
            ]
        )
            .into()
    );

    Ok(())
}
