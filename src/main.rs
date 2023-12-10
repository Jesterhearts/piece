#![allow(clippy::single_match)]

use std::collections::HashMap;

use anyhow::{anyhow, Context};
use include_dir::{include_dir, Dir};
use indoc::indoc;
use rusqlite::Connection;

use crate::{
    battlefield::Battlefield,
    card::Card,
    effects::Counter,
    in_play::{CardId, CounterId},
    player::AllPlayers,
};

pub mod abilities;
pub mod battlefield;
pub mod card;
pub mod controller;
pub mod cost;
pub mod deck;
pub mod effects;
pub mod hand;
pub mod in_play;
pub mod mana;
pub mod player;
pub mod protogen;
pub mod stack;
pub mod targets;
pub mod types;

#[cfg(test)]
pub mod tests;
pub mod triggers;

static CARD_DEFINITIONS: Dir = include_dir!("cards");

pub type Cards = HashMap<String, Card>;

pub fn load_cards() -> anyhow::Result<Cards> {
    let mut cards = Cards::default();
    for card in CARD_DEFINITIONS.entries().iter() {
        let card_file = card
            .as_file()
            .ok_or_else(|| anyhow!("Non-file entry in cards directory"))?;

        let card: protogen::card::Card = protobuf::text_format::parse_from_str(
            card_file
                .contents_utf8()
                .ok_or_else(|| anyhow!("Non utf-8 text proto"))?,
        )
        .with_context(|| format!("Parsing file: {}", card_file.path().display()))?;

        cards.insert(
            card.name.to_owned(),
            card.try_into()
                .with_context(|| format!("Validating file: {}", card_file.path().display()))?,
        );
    }

    Ok(cards)
}

fn prepare_db() -> anyhow::Result<Connection> {
    let db = Connection::open_in_memory()?;

    db.execute(
        indoc! {"
            CREATE TABLE auras (
                auraid INTEGER PRIMARY KEY,
                modifiers JSON,

                restrictions JSON
            );"},
        (),
    )?;

    db.execute(
        indoc! { "
        CREATE TABLE cards (
            cardid INTEGER PRIMARY KEY,
            aura INTEGER,

            marked_damage INTEGER NOT NULL,

            cloning INTEGER,

            location JSON NOT NULL,
            location_seq INTEGER,

            name TEXT NOT NULL,

            owner INTEGER NOT NULL,
            controller INTEGER NOT NULL,

            tapped BOOLEAN NOT NULL,
            manifested BOOLEAN NOT NULL,
            face_down BOOLEAN NOT NULL,
            token BOOLEAN NOT NULL,

            casting_cost JSON,
            cannot_be_countered BOOLEAN NOT NULL,
            split_second BOOLEAN NOT NULL,

            effects JSON,

            power INTEGER,
            toughness INTEGER,

            types JSON,
            subtypes JSON,
            
            colors JSON,

            etb JSON,
            abilities JSON,
            activated_abilities JSON,
            triggered_abilities JSON,

            vigilance BOOLEAN,
            flying BOOLEAN,
            flash BOOLEAN,
            hexproof BOOLEAN,
            shroud BOOLEAN,

            targets JSON,
            mode INTEGER,
            
            FOREIGN KEY(aura) REFERENCES auras(auraid)
        );"},
        (),
    )?;

    db.execute(
        indoc! { "
        CREATE TABLE modifiers (
            modifierid INTEGER PRIMARY KEY,
            source INTEGER,

            is_temporary BOOLEAN NOT NULL,
            
            type_modifiers JSON,
            subtype_modifiers JSON,
            remove_all_subtypes BOOLEAN,
            
            color_modifiers JSON,
            
            ability_modifiers JSON,
            
            base_power_modifier INTEGER,
            base_toughness_modifier INTEGER,

            dynamic_add_power_toughess JSON,
            
            add_power_modifier INTEGER,
            add_toughness_modifier INTEGER,

            activated_ability_modifier JSON,
            static_ability_modifier JSON,
            triggered_ability_modifier JSON,
            
            add_vigilance BOOLEAN,
            remove_vigilance BOOLEAN,

            add_flying BOOLEAN,
            remove_flying BOOLEAN,

            add_flash BOOLEAN,
            remove_flash BOOLEAN,

            add_hexproof BOOLEAN,
            remove_hexproof BOOLEAN,

            add_shroud BOOLEAN,
            remove_shroud BOOLEAN,
            
            controller JSON NOT NULL,
            duration JSON NOT NULL,
            restrictions JSON NOT NULL,

            global BOOLEAN NOT NULL,
            entire_battlefield BOOLEAN NOT NULL,

            active BOOLEAN NOT NULL,
            active_seq INTEGER,

            modifying JSON,

            FOREIGN KEY(source) REFERENCES cards(cardid)
        );"},
        (),
    )?;

    db.execute(
        indoc! {"
            CREATE TABLE triggers (
                triggerid INTEGER PRIMARY KEY,
                listener INTEGER NOT NULL,

                source JSON NOT NULL,
                location_from JSON NOT NULL,
                for_types JSON NOT NULL,

                effects JSON NOT NULL,

                active BOOLEAN NOT NULL,

                in_stack BOOLEAN NOT NULL,
                stack_seq INTEGER,

                targets JSON,
                mode INTEGER,

                FOREIGN KEY(listener) REFERENCES cards(cardid)
            );"},
        (),
    )?;

    db.execute(
        indoc! {"
            CREATE TABLE abilities (
                abilityid INTEGER PRIMARY KEY,
                source INTEGER,

                cost JSON NOT NULL,
                effects JSON NOT NULL,

                apply_to_self BOOLEAN NOT NULL,

                in_stack BOOLEAN NOT NULL,
                stack_seq INTEGER,

                targets JSON,
                mode INTEGER,

                FOREIGN KEY(source) REFERENCES cards(cardid)
            )
        "},
        (),
    )?;

    db.execute(
        indoc! {"
            CREATE TABLE counters (
                counterid INTEGER PRIMARY KEY,
                is_on INTEGER NOT NULL,

                type JSON NOT NULL,
                count INTEGER NOT NULL,

                FOREIGN KEY(is_on) REFERENCES cards(cardid)
            )
        "},
        (),
    )?;

    Ok(db)
}

fn main() -> anyhow::Result<()> {
    let cards = load_cards()?;
    dbg!(&cards);

    let db = prepare_db()?;

    let mut all_players = AllPlayers::default();

    let player = all_players.new_player();

    let card1 = CardId::upload(&db, &cards, player, "Mace of the Valiant")?;

    CounterId::add_counters(&db, card1, Counter::P1P1, 1)?;
    dbg!(CounterId::counters_on(&db, card1, Counter::P1P1))?;

    Ok(())
}
