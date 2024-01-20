use std::collections::HashMap;

use convert_case::{Case, Casing};
use itertools::Itertools;
use piece_lib::protogen::{
    card::Card,
    cost::ManaCost,
    types::{Subtype, Type},
};
use protobuf::Enum;
use serde_json::Value;

#[allow(clippy::field_reassign_with_default)]
fn main() -> anyhow::Result<()> {
    let card_file = std::fs::read("experimental/oracle-cards.json")?;

    let value: Value = serde_json::from_slice(&card_file)?;
    let mut unique_cards: HashMap<String, Card> = HashMap::default();
    for card in value.as_array().unwrap().iter() {
        if card["legalities"]["commander"] != Value::String("legal".to_string()) {
            continue;
        }
        if !card["games"]
            .as_array()
            .unwrap()
            .contains(&Value::String("paper".to_string()))
        {
            continue;
        }
        if card["set_type"] == Value::String("funny".to_string()) {
            continue;
        }

        if let Some(faces) = card["card_faces"].as_array() {
            let mut proto = Card::default();

            if let Some(cost) = parse_mana_cost(faces[0]["mana_cost"].as_str().unwrap()) {
                proto.cost.mut_or_insert_default().mana_cost = cost;
            } else {
                continue;
            }

            if let Some(cost) = parse_mana_cost(faces[1]["mana_cost"].as_str().unwrap()) {
                proto.cost.mut_or_insert_default().mana_cost = cost;
            } else {
                continue;
            }

            let (types, subtypes) = parse_typeline(faces[0]["type_line"].as_str().unwrap());
            proto.typeline.mut_or_insert_default().types = types;
            proto.typeline.mut_or_insert_default().subtypes = subtypes;

            proto.name = faces[0]["name"].as_str().unwrap().to_string();
            proto.oracle_text = faces[0]["oracle_text"].as_str().unwrap().to_string();

            let (types, subtypes) = parse_typeline(faces[1]["type_line"].as_str().unwrap());
            proto.typeline.mut_or_insert_default().types = types;
            proto.typeline.mut_or_insert_default().subtypes = subtypes;
            proto.name = faces[1]["name"].as_str().unwrap().to_string();
            proto.oracle_text = faces[1]["oracle_text"].as_str().unwrap().to_string();

            unique_cards.insert(proto.name.clone(), proto);
        } else {
            let mut proto = Card::default();

            if let Some(cost) = parse_mana_cost(card["mana_cost"].as_str().unwrap()) {
                proto.cost.mut_or_insert_default().mana_cost = cost;
            } else {
                continue;
            }

            let (types, subtypes) = parse_typeline(card["type_line"].as_str().unwrap());
            proto.typeline.mut_or_insert_default().types = types;
            proto.typeline.mut_or_insert_default().subtypes = subtypes;
            proto.name = card["name"].as_str().unwrap().to_string();
            proto.oracle_text = card["oracle_text"].as_str().unwrap().to_string();

            if let Some(power) = card["power"].as_str() {
                let Ok(power) = power.parse::<i32>() else {
                    eprintln!("{}", power);
                    continue;
                };

                proto.power = Some(power);
            }

            if let Some(toughness) = card["toughness"].as_str() {
                let Ok(toughness) = toughness.parse::<i32>() else {
                    eprintln!("{}", toughness);
                    continue;
                };

                proto.toughness = Some(toughness);
            }

            let mut name = proto.name.replace("+", "plus_");
            name.retain(|c| !['-', '\'', ',', '+', '"'].contains(&c));
            unique_cards.insert(name, proto);
        }
    }

    println!("{} cards", unique_cards.len());
    for (title, card) in unique_cards {
        let root = std::path::Path::new("experimental/cards").join(
            title
                .from_case(Case::Title)
                .to_case(Case::Snake)
                .trim_start_matches("the_")
                .chars()
                .take(1)
                .join(""),
        );
        let _ = std::fs::create_dir_all(&root);

        std::fs::write(
            root.join(title.from_case(Case::Title).to_case(Case::Snake))
                .with_extension("yaml"),
            serde_yaml::to_string(&card)?,
        )?;
    }

    Ok(())
}

fn parse_typeline(
    s: &str,
) -> (
    Vec<protobuf::EnumOrUnknown<Type>>,
    Vec<protobuf::EnumOrUnknown<Subtype>>,
) {
    let type_and_subtype = s
        .split('—')
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.split_ascii_whitespace()
                .map(|s| s.split(&['-', '\'']).join(""))
                .collect_vec()
        })
        .collect_vec();

    let mut types = vec![];
    let mut subtypes = vec![];

    match type_and_subtype.as_slice() {
        [ty] => {
            for s in ty.iter() {
                let Some(ty) = Type::from_str(&s.to_case(Case::ScreamingSnake)) else {
                    unreachable!();
                };
                types.push(ty)
            }
        }
        [ty, subty] => {
            for s in ty.iter() {
                let Some(ty) = Type::from_str(&s.to_case(Case::ScreamingSnake)) else {
                    unreachable!();
                };
                types.push(ty)
            }

            for s in subty.iter() {
                let Some(ty) = Subtype::from_str(&s.to_case(Case::ScreamingSnake)) else {
                    eprintln!("{}", s);
                    continue;
                };

                subtypes.push(ty)
            }
        }
        other => unreachable!("{:?}", other),
    }

    (
        types
            .into_iter()
            .map(protobuf::EnumOrUnknown::new)
            .collect_vec(),
        subtypes
            .into_iter()
            .map(protobuf::EnumOrUnknown::new)
            .collect_vec(),
    )
}

fn parse_mana_cost(v: &str) -> Option<Vec<protobuf::EnumOrUnknown<ManaCost>>> {
    let split = v
        .split('}')
        .map(|s| s.trim_start_matches('{'))
        .filter(|s| !s.is_empty())
        .collect_vec();

    let mut results = vec![];
    for symbol in split {
        if let Ok(count) = symbol.parse::<usize>() {
            for _ in 0..count {
                results.push(ManaCost::GENERIC);
            }
        } else {
            let cost = match symbol {
                "W" => ManaCost::WHITE,
                "U" => ManaCost::BLUE,
                "B" => ManaCost::BLACK,
                "R" => ManaCost::RED,
                "G" => ManaCost::GREEN,
                "X" => ManaCost::X,
                "C" => ManaCost::COLORLESS,
                _ => {
                    return None;
                }
            };

            if matches!(cost, ManaCost::X) {
                if matches!(results.last(), Some(ManaCost::X)) {
                    results.pop();
                    results.push(ManaCost::TWO_X);
                } else if matches!(results.last(), Some(ManaCost::TWO_X)) {
                    return None;
                } else {
                    results.push(cost)
                }
            } else {
                results.push(cost);
            }
        }
    }

    Some(
        results
            .into_iter()
            .map(protobuf::EnumOrUnknown::new)
            .collect_vec(),
    )
}
