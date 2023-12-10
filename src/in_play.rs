use std::{
    cell::OnceCell,
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicUsize, Ordering},
};

use derive_more::From;
use indoc::indoc;
use rusqlite::{types::FromSql, Connection, ToSql};
use serde::{Deserialize, Serialize};

use crate::{
    abilities::{ActivatedAbility, ETBAbility, StaticAbility, TriggeredAbility},
    battlefield::Battlefield,
    card::{
        ActivatedAbilityModifier, Card, Color, StaticAbilityModifier, TriggeredAbilityModifier,
    },
    controller::Controller,
    cost::{AbilityCost, CastingCost},
    effects::{
        AnyEffect, BattlefieldModifier, Counter, DealDamage, DynamicPowerToughness, Effect,
        EffectDuration, GainMana, Token,
    },
    mana::Mana,
    player::PlayerId,
    stack::{ActiveTarget, Stack},
    targets::{Comparison, Restriction, SpellTarget},
    triggers::{self, Trigger, TriggerSource},
    types::{Subtype, Type},
    Cards,
};

static NEXT_CARD_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_MODIFIER_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_ABILITY_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_AURA_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_TRIGGER_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_COUNTER_ID: AtomicUsize = AtomicUsize::new(0);

static NEXT_MODIFIER_SEQ: AtomicUsize = AtomicUsize::new(0);
/// Starts at 1 because 0 should never be a valid stack id.
static NEXT_STACK_SEQ: AtomicUsize = AtomicUsize::new(1);
static NEXT_GRAVEYARD_SEQ: AtomicUsize = AtomicUsize::new(0);
static NEXT_HAND_SEQ: AtomicUsize = AtomicUsize::new(0);
static NEXT_BATTLEFIELD_SEQ: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static INIT_LAND_ABILITIES: OnceCell<HashMap<Subtype, AbilityId>> = OnceCell::new();
}

static UPLOAD_CARD_SQL: &str = indoc! {"
    INSERT INTO cards (
        cardid,
        marked_damage,
        location,
        name,
        owner,
        controller,
        tapped,
        manifested,
        face_down,
        token,
        cannot_be_countered,
        split_second
    ) VALUES (
        (?1),
        (?2),
        (?3),
        (?4),
        (?5),
        (?6),
        (?7),
        (?8),
        (?9),
        (?10),
        (?11),
        (?12)
    );
"};

static UPLOAD_MODIFIER_SQL: &str = indoc! {"
    INSERT INTO modifiers (
        modifierid,
        duration,
        is_temporary,
        controller,
        restrictions,
        global,
        entire_battlefield,
        active
    ) VALUES (
        (?1),
        (?2),
        (?3),
        (?4),
        (?5),
        (?6),
        (?7),
        (?8)
    )
"};

static INSERT_ABILITIES_SQL: &str = indoc! {"
    INSERT INTO abilities (
        abilityid,
        source,
        apply_to_self,
        cost,
        effects,
        in_stack
    ) VALUES (
        (?1),
        (?2),
        (?3),
        (?4),
        (?5),
        (?6)
    )"
};

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum Location {
    Library,
    Hand,
    Stack,
    Battlefield,
    Graveyard,
    Exile,
}

impl Location {
    pub fn cards_in(&self, db: &Connection) -> anyhow::Result<Vec<CardId>> {
        let mut results = vec![];
        let mut in_location = db.prepare(indoc! {"
            SELECT cardid
            FROM cards
            WHERE location = (?1)
            ORDER BY location_seq ASC
        "})?;

        for row in in_location.query_map((serde_json::to_string(self)?,), |row| row.get(0))? {
            results.push(row?)
        }

        Ok(results)
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Copy, Hash, Default, From)]
pub struct CardId(usize);

impl FromSql for CardId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(Self(usize::column_result(value)?))
    }
}

impl ToSql for CardId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

impl CardId {
    pub fn new() -> Self {
        Self(NEXT_CARD_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn is_in_location(self, db: &Connection, location: Location) -> anyhow::Result<bool> {
        let mut in_location = db.prepare(indoc! {"
            SELECT NULL
            FROM cards
            WHERE cardid = (?1)
                AND location = (?2)
        "})?;

        let mut in_location = in_location.query((self, serde_json::to_string(&location)?))?;
        Ok(in_location.next()?.is_some())
    }

    pub fn move_to_hand(self, db: &Connection) -> anyhow::Result<()> {
        self.remove_all_modifiers(db)?;
        TriggerId::deactivate_all_for_card(db, self)?;
        self.deactivate_modifiers(db)?;

        db.execute(
            indoc! { "
                UPDATE cards
                SET location = (?2),
                    location_seq = (?3),
                    controller = owner
                WHERE cards.cardid = (?1)
            "},
            (
                self,
                serde_json::to_string(&Location::Hand)?,
                NEXT_HAND_SEQ.fetch_add(1, Ordering::Relaxed),
            ),
        )?;

        // TODO tokens go poof

        Ok(())
    }

    pub fn move_to_stack(
        self,
        db: &Connection,
        targets: HashSet<ActiveTarget>,
    ) -> anyhow::Result<()> {
        if Stack::split_second(db)? {
            return Ok(());
        }

        db.execute(
            indoc! { "
                UPDATE cards
                SET location = (?2),
                    location_seq = (?3),
                    targets = (?4)
                WHERE cards.cardid = (?1)
            "},
            (
                self,
                serde_json::to_string(&Location::Stack)?,
                NEXT_STACK_SEQ.fetch_add(1, Ordering::Relaxed),
                serde_json::to_string(&targets)?,
            ),
        )?;

        Ok(())
    }

    pub fn move_to_battlefield(self, db: &Connection) -> anyhow::Result<()> {
        db.execute(
            "UPDATE cards SET location = (?2), location_seq = (?3) WHERE cards.cardid = (?1)",
            (
                self,
                serde_json::to_string(&Location::Battlefield)?,
                NEXT_BATTLEFIELD_SEQ.fetch_add(1, Ordering::Relaxed),
            ),
        )?;

        TriggerId::activate_all_for_card(db, self)?;

        Ok(())
    }

    pub fn move_to_graveyard(self, db: &Connection) -> anyhow::Result<()> {
        self.remove_all_modifiers(db)?;
        TriggerId::deactivate_all_for_card(db, self)?;
        self.deactivate_modifiers(db)?;

        db.execute(
            indoc! { "
                UPDATE cards
                SET location = (?2),
                    location_seq = (?3),
                    controller = owner
                WHERE cards.cardid = (?1)
            "},
            (
                self,
                serde_json::to_string(&Location::Graveyard)?,
                NEXT_GRAVEYARD_SEQ.fetch_add(1, Ordering::Relaxed),
            ),
        )?;

        // TODO tokens go poof

        Ok(())
    }

    pub fn move_to_library(self, db: &Connection) -> anyhow::Result<()> {
        self.remove_all_modifiers(db)?;
        TriggerId::deactivate_all_for_card(db, self)?;
        self.deactivate_modifiers(db)?;

        db.execute(
            indoc! { "
                UPDATE cards
                SET location = (?2),
                    location_seq = (?3),
                    controller = owner
                WHERE cards.cardid = (?1)
            "},
            (
                self,
                serde_json::to_string(&Location::Library)?,
                NEXT_GRAVEYARD_SEQ.fetch_add(1, Ordering::Relaxed),
            ),
        )?;

        // TODO tokens go poof

        Ok(())
    }

    pub fn move_to_exile(self, db: &Connection) -> anyhow::Result<()> {
        self.remove_all_modifiers(db)?;
        TriggerId::deactivate_all_for_card(db, self)?;
        self.deactivate_modifiers(db)?;

        db.execute(
            "UPDATE cards SET location = (?2), controller = owner WHERE cards.cardid = (?1)",
            (self, serde_json::to_string(&Location::Exile)?),
        )?;

        // TODO tokens go poof

        Ok(())
    }

    pub fn remove_all_modifiers(self, db: &Connection) -> anyhow::Result<()> {
        let mut statement = db.prepare(indoc! {"
                SELECT modifierid, modifying
                FROM modifiers, json_each(modifiers.modifying)
                WHERE json_each.value = (?1)
            "})?;

        let rows = statement.query_map((self,), |row| {
            Ok((
                row.get::<_, ModifierId>(0)?,
                serde_json::from_str::<HashSet<CardId>>(&row.get::<_, String>(1)?).unwrap(),
            ))
        })?;

        for row in rows {
            let (modifierid, mut modifying) = row?;
            modifying.remove(&self);

            db.execute(
                indoc! { "
                    UPDATE modifiers
                    SET modifying = (?2)
                    WHERE modifiers.modifierid = (?1)
                "},
                (modifierid, serde_json::to_string(&modifying)?),
            )?;

            modifierid.deactivate(db)?;
        }

        Ok(())
    }

    pub fn remove_modifier(self, db: &Connection, modifier: ModifierId) -> anyhow::Result<()> {
        let mut modifying = db.query_row(
            indoc! {"
                SELECT modifying
                FROM modifiers 
                WHERE modifiers.modifierid = (?1)
            "},
            (modifier,),
            |row| Ok(serde_json::from_str::<HashSet<CardId>>(&row.get::<_, String>(0)?).unwrap()),
        )?;

        modifying.remove(&self);
        db.execute(
            "UPDATE modifiers SET modifying = (?2) WHERE modifierid = (?1)",
            (modifier, serde_json::to_string(&modifying)?),
        )?;

        modifier.deactivate(db)?;

        Ok(())
    }

    pub fn modifiers(self, db: &Connection) -> anyhow::Result<Vec<ModifierId>> {
        let mut statement = db.prepare(indoc! {"
                SELECT modifierid
                FROM modifiers, json_each(modifiers.modifying)
                WHERE json_each.value = (?1)
            "})?;

        let rows = statement.query_map((self,), |row| row.get(0))?;

        let mut modifiers = vec![];
        for row in rows {
            modifiers.push(row?);
        }

        Ok(modifiers)
    }

    pub fn deactivate_modifiers(&self, db: &Connection) -> anyhow::Result<()> {
        let mut statement = db.prepare(indoc! {"
                SELECT modifierid
                FROM modifiers
                WHERE source = (?1)
                    AND duration = (?2)
            "})?;

        let rows = statement.query_map(
            (
                self,
                serde_json::to_string(&EffectDuration::UntilSourceLeavesBattlefield)?,
            ),
            |row| row.get::<_, ModifierId>(0),
        )?;

        for row in rows {
            row?.detach_all(db)?;
        }

        Ok(())
    }

    pub fn triggered_abilities(self, db: &Connection) -> anyhow::Result<Vec<TriggerId>> {
        let face_down: bool = db.query_row(
            "SELECT face_down FROM cards WHERE cardid = (?1)",
            (self,),
            |row| row.get(0),
        )?;

        let mut abilities: Vec<TriggerId> = if face_down {
            vec![]
        } else {
            db.query_row(
                "SELECT triggered_abilities FROM cards WHERE cardid = (?1)",
                (if let Some(cloning) = self.cloning(db)? {
                    cloning
                } else {
                    self
                },),
                |row| {
                    Ok(row
                        .get::<_, Option<String>>(0)?
                        .map(|row| serde_json::from_str(&row).unwrap())
                        .unwrap_or_default())
                },
            )?
        };

        let mut statement = db.prepare(indoc! {"
                SELECT triggered_ability_modifier, source, controller, restrictions, active_seq
                FROM modifiers
                WHERE active AND (
                    global
                    OR entire_battlefield
                )
                UNION
                SELECT triggered_ability_modifier, source, controller, restrictions, active_seq
                FROM modifiers, json_each(modifiers.modifying)
                WHERE active AND json_each.value = (?1)
                ORDER BY active_seq ASC
            "})?;

        let rows = statement.query_map((self,), |row| {
            Ok((
                row.get::<_, Option<String>>(0)?
                    .map(|row| serde_json::from_str(&row).unwrap()),
                row.get(1)?,
                serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                serde_json::from_str::<Vec<_>>(&row.get::<_, String>(3)?).unwrap(),
            ))
        })?;

        for row in rows {
            let (modifier, source, controller_restriction, restrictions) = row?;

            if self.passes_restrictions(
                db,
                source,
                source.controller(db)?,
                controller_restriction,
                &restrictions,
            )? {
                if let Some(modifier) = modifier {
                    match modifier {
                        TriggeredAbilityModifier::RemoveAll => abilities.clear(),
                        TriggeredAbilityModifier::Add(ability) => {
                            abilities.push(ability);
                        }
                    }
                }
            }
        }

        Ok(abilities)
    }

    pub fn etb_abilities(self, db: &Connection) -> anyhow::Result<Vec<ETBAbility>> {
        let face_down: bool = db.query_row(
            "SELECT face_down FROM cards WHERE cardid = (?1)",
            (self,),
            |row| row.get(0),
        )?;

        if face_down {
            return Ok(vec![]);
        }

        Ok(db.query_row(
            "SELECT etb FROM cards WHERE cardid = (?1)",
            (if let Some(cloning) = self.cloning(db)? {
                cloning
            } else {
                self
            },),
            |row| {
                Ok(row
                    .get::<_, Option<String>>(0)?
                    .map(|row| serde_json::from_str(&row).unwrap())
                    .unwrap_or_default())
            },
        )?)
    }

    pub fn static_abilities(self, db: &Connection) -> anyhow::Result<Vec<StaticAbility>> {
        let face_down: bool = db.query_row(
            "SELECT face_down FROM cards WHERE cardid = (?1)",
            (self,),
            |row| row.get(0),
        )?;

        let mut abilities: Vec<StaticAbility> = if face_down {
            vec![]
        } else {
            db.query_row(
                "SELECT abilities FROM cards WHERE cardid = (?1)",
                (if let Some(cloning) = self.cloning(db)? {
                    cloning
                } else {
                    self
                },),
                |row| {
                    Ok(row
                        .get::<_, Option<String>>(0)?
                        .map(|row| serde_json::from_str(&row).unwrap())
                        .unwrap_or_default())
                },
            )?
        };

        let mut statement = db.prepare(indoc! {"
                SELECT static_ability_modifier, source, controller, restrictions, active_seq
                FROM modifiers
                WHERE active AND (
                    global
                    OR entire_battlefield
                )
                UNION
                SELECT static_ability_modifier, source, controller, restrictions, active_seq
                FROM modifiers, json_each(modifiers.modifying)
                WHERE active AND json_each.value = (?1)
                ORDER BY active_seq ASC
            "})?;

        let rows = statement.query_map((self,), |row| {
            Ok((
                row.get::<_, Option<String>>(0)?
                    .map(|row| serde_json::from_str(&row).unwrap()),
                row.get(1)?,
                serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                serde_json::from_str::<Vec<_>>(&row.get::<_, String>(3)?).unwrap(),
            ))
        })?;

        for row in rows {
            let (modifier, source, controller_restriction, restrictions) = row?;

            if self.passes_restrictions(
                db,
                source,
                source.controller(db)?,
                controller_restriction,
                &restrictions,
            )? {
                if let Some(modifier) = modifier {
                    match modifier {
                        StaticAbilityModifier::RemoveAll => abilities.clear(),
                        StaticAbilityModifier::Add(ability) => {
                            abilities.push(ability);
                        }
                    }
                }
            }
        }

        Ok(abilities)
    }

    pub fn activated_abilities(self, db: &Connection) -> anyhow::Result<Vec<AbilityId>> {
        let face_down: bool = db.query_row(
            "SELECT face_down FROM cards WHERE cardid = (?1)",
            (self,),
            |row| row.get(0),
        )?;

        let mut abilities: Vec<AbilityId> = if face_down {
            vec![]
        } else {
            db.query_row(
                "SELECT activated_abilities FROM cards WHERE cardid = (?1)",
                (if let Some(cloning) = self.cloning(db)? {
                    cloning
                } else {
                    self
                },),
                |row| {
                    Ok(row
                        .get::<_, Option<String>>(0)?
                        .map(|row| serde_json::from_str(&row).unwrap())
                        .unwrap_or_default())
                },
            )?
        };

        let mut statement = db.prepare(indoc! {"
                SELECT activated_ability_modifier, source, controller, restrictions, active_seq FROM modifiers
                WHERE active AND (
                    global
                    OR entire_battlefield
                )
                UNION
                SELECT activated_ability_modifier, source, controller, restrictions, active_seq
                FROM modifiers, json_each(modifiers.modifying)
                WHERE active AND json_each.value = (?1)
                ORDER BY active_seq ASC
            "})?;

        let rows = statement.query_map((self,), |row| {
            Ok((
                row.get::<_, Option<String>>(0)?
                    .map(|row| serde_json::from_str(&row).unwrap()),
                row.get(1)?,
                serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                serde_json::from_str::<Vec<_>>(&row.get::<_, String>(3)?).unwrap(),
            ))
        })?;

        for row in rows {
            let (modifier, source, controller_restriction, restrictions) = row?;

            if self.passes_restrictions(
                db,
                source,
                source.controller(db)?,
                controller_restriction,
                &restrictions,
            )? {
                if let Some(modifier) = modifier {
                    match modifier {
                        ActivatedAbilityModifier::RemoveAll => abilities.clear(),
                        ActivatedAbilityModifier::Add(ability) => {
                            abilities.push(ability);
                        }
                    }
                }
            }
        }

        let land_abilities = AbilityId::land_abilities(db);
        for ty in self.subtypes(db)? {
            if let Some(ability) = land_abilities.get(&ty) {
                abilities.push(*ability);
            }
        }

        Ok(abilities)
    }

    pub fn controller(self, db: &Connection) -> anyhow::Result<PlayerId> {
        Ok(db.query_row(
            "SELECT controller FROM cards WHERE cardid = (?1)",
            (self,),
            |row| row.get(0),
        )?)
    }

    pub fn owner(self, db: &Connection) -> anyhow::Result<PlayerId> {
        Ok(db.query_row(
            "SELECT owner FROM cards WHERE cardid = (?1)",
            (self,),
            |row| row.get(0),
        )?)
    }

    pub fn apply_modifier(self, db: &Connection, modifier: ModifierId) -> anyhow::Result<()> {
        let modifying = db.query_row(
            "SELECT modifying FROM modifiers WHERE modifierid = (?1)",
            (modifier,),
            |row| row.get::<_, Option<String>>(0),
        )?;

        let modifying = if let Some(modifying) = modifying {
            let mut modifying: Vec<CardId> = serde_json::from_str(&modifying)?;
            modifying.push(self);
            modifying
        } else {
            vec![self]
        };

        db.execute(
            indoc! {"
                UPDATE modifiers
                SET modifying = (?2)
                WHERE modifierid = (?1)
            "},
            (modifier, serde_json::to_string(&modifying)?),
        )?;

        modifier.activate(db)?;

        Ok(())
    }

    pub fn effects(self, db: &Connection) -> anyhow::Result<Vec<AnyEffect>> {
        Ok(db.query_row(
            "SELECT effects FROM cards WHERE cardid = (?1)",
            (if let Some(cloning) = self.cloning(db)? {
                cloning
            } else {
                self
            },),
            |row| {
                Ok(row
                    .get::<_, Option<String>>(0)?
                    .map(|row| serde_json::from_str(&row).unwrap())
                    .unwrap_or_default())
            },
        )?)
    }

    pub fn passes_restrictions(
        self,
        db: &Connection,
        source: CardId,
        controller: PlayerId,
        controller_restriction: Controller,
        restrictions: &[Restriction],
    ) -> Result<bool, anyhow::Error> {
        self.passes_restrictions_given_types(
            db,
            source,
            controller,
            controller_restriction,
            restrictions,
            &self.types(db)?,
            &self.subtypes(db)?,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn passes_restrictions_given_types(
        self,
        db: &Connection,
        source: CardId,
        controller: PlayerId,
        controller_restriction: Controller,
        restrictions: &[Restriction],
        self_types: &HashSet<Type>,
        self_subtypes: &HashSet<Subtype>,
    ) -> Result<bool, anyhow::Error> {
        match controller_restriction {
            Controller::Any => {}
            Controller::You => {
                let source_controller = source.controller(db)?;
                if source_controller != controller {
                    return Ok(false);
                }
            }
            Controller::Opponent => {
                let source_controller = source.controller(db)?;
                if source_controller == controller {
                    return Ok(false);
                }
            }
        }

        for restriction in restrictions.iter() {
            match restriction {
                Restriction::NotSelf => {
                    if source == self {
                        return Ok(false);
                    }
                }
                Restriction::Self_ => {
                    if source != self {
                        return Ok(false);
                    }
                }
                Restriction::OfType { types, subtypes } => {
                    if !types.is_empty() && self_types.is_disjoint(types) {
                        return Ok(false);
                    }

                    if !subtypes.is_empty() && self_subtypes.is_disjoint(subtypes) {
                        return Ok(false);
                    }
                }
                Restriction::Toughness(comparison) => {
                    let toughness = self.toughness(db)?;
                    if toughness.is_none() {
                        return Ok(false);
                    }

                    let toughness = toughness.unwrap();
                    if !match comparison {
                        Comparison::LessThan(target) => toughness < *target,
                        Comparison::LessThanOrEqual(target) => toughness <= *target,
                    } {
                        return Ok(false);
                    }
                }
                Restriction::ControllerControlsBlackOrGreen => {
                    let controller = self.controller(db)?;
                    let colors = Battlefield::controlled_colors(db, controller)?;
                    if !(colors.contains(&Color::Green) || colors.contains(&Color::Black)) {
                        return Ok(false);
                    }
                }
            }
        }

        Ok(true)
    }

    pub fn apply_aura(self, db: &Connection, aura: AuraId) -> anyhow::Result<()> {
        let modifiers = aura.modifiers(db)?;

        for modifier in modifiers {
            self.apply_modifier(db, modifier)?;
        }

        Ok(())
    }

    pub fn marked_damage(self, db: &Connection) -> anyhow::Result<i32> {
        Ok(db.query_row(
            "SELECT marked_damage FROM cards WHERE cardid = (?1)",
            (self,),
            |row| row.get(0),
        )?)
    }

    pub fn mark_damage(self, db: &Connection, amount: usize) -> anyhow::Result<()> {
        db.execute(
            "UPDATE cards SET marked_damage = marked_damage + (?2) WHERE cardid = (?1)",
            (self, amount),
        )?;
        Ok(())
    }

    pub fn clear_damage(self, db: &Connection) -> anyhow::Result<()> {
        db.execute(
            "UPDATE cards SET marked_damage = 0 WHERE cardid = (?1)",
            (self,),
        )?;
        Ok(())
    }

    pub fn power(self, db: &Connection) -> anyhow::Result<Option<i32>> {
        let face_down: bool = db.query_row(
            "SELECT face_down FROM cards WHERE cardid = (?1)",
            (self,),
            |row| row.get(0),
        )?;

        if face_down {
            return Ok(Some(2));
        }

        let mut base: Option<i32> = db.query_row(
            "SELECT power FROM cards WHERE cardid = (?1)",
            (if let Some(cloning) = self.cloning(db)? {
                cloning
            } else {
                self
            },),
            |row| row.get(0),
        )?;

        let mut statement = db.prepare(indoc! {"
                SELECT
                    base_power_modifier,
                    dynamic_add_power_toughness,
                    add_power_modifier,
                    source,
                    controller,
                    restrictions,
                    active_seq
                FROM modifiers
                WHERE active AND (
                    global
                    OR entire_battlefield
                )
                UNION
                SELECT base_power_modifier, add_power_modifier, source, controller, restrictions, active_seq
                FROM modifiers, json_each(modifiers.modifying)
                WHERE active AND json_each.value = (?1)
                ORDER BY active_seq ASC
            "})?;

        let rows = statement.query_map((self,), |row| {
            Ok((
                row.get::<_, Option<i32>>(0)?,
                row.get::<_, Option<String>>(1)?
                    .map(|col| serde_json::from_str::<DynamicPowerToughness>(&col).unwrap()),
                row.get::<_, Option<i32>>(2)?,
                row.get(3)?,
                serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
                serde_json::from_str::<Vec<_>>(&row.get::<_, String>(5)?).unwrap(),
            ))
        })?;

        let mut add = 0;

        for row in rows {
            let (base_mod, dynamic_add_mod, add_mod, source, controller_restriction, restrictions) =
                row?;

            if self.passes_restrictions(
                db,
                source,
                source.controller(db)?,
                controller_restriction,
                &restrictions,
            )? {
                if let Some(base_mod) = base_mod {
                    base = Some(base_mod);
                }
                add += add_mod.unwrap_or_default();

                if let Some(dynamic) = dynamic_add_mod {
                    match dynamic {
                        DynamicPowerToughness::NumberOfCountersOnThis(counter) => {
                            let to_add = CounterId::counters_on(db, source, counter)?;
                            add += to_add as i32;
                        }
                    }
                }
            }
        }

        Ok(base.map(|base| base + add))
    }

    pub fn toughness(self, db: &Connection) -> anyhow::Result<Option<i32>> {
        let face_down: bool = db.query_row(
            "SELECT face_down FROM cards WHERE cardid = (?1)",
            (self,),
            |row| row.get(0),
        )?;

        if face_down {
            return Ok(Some(2));
        }

        let mut base: Option<i32> = db.query_row(
            "SELECT toughness FROM cards WHERE cardid = (?1)",
            (if let Some(cloning) = self.cloning(db)? {
                cloning
            } else {
                self
            },),
            |row| row.get(0),
        )?;

        let mut statement = db.prepare(indoc! {"
                SELECT base_toughness_modifier, add_toughness_modifier, source, controller, restrictions, active_seq
                FROM modifiers
                WHERE active AND (
                    global
                    OR entire_battlefield
                )
                UNION
                SELECT base_toughness_modifier, add_toughness_modifier, source, controller, restrictions, active_seq
                FROM modifiers, json_each(modifiers.modifying)
                WHERE active AND json_each.value = (?1)
                ORDER BY active_seq ASC
            "})?;

        let rows = statement.query_map((self,), |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
            ))
        })?;

        let mut add = 0;

        for row in rows {
            let (base_mod, add_mod, source, controller_restriction, restrictions): (
                Option<i32>,
                Option<i32>,
                _,
                _,
                Vec<Restriction>,
            ) = row?;

            if self.passes_restrictions(
                db,
                source,
                source.controller(db)?,
                controller_restriction,
                &restrictions,
            )? {
                if let Some(base_mod) = base_mod {
                    base = Some(base_mod);
                }
                add += add_mod.unwrap_or_default();
            }
        }

        Ok(base.map(|base| base + add))
    }

    pub fn vigilance(&self, db: &Connection) -> anyhow::Result<bool> {
        let face_down: bool = db.query_row(
            "SELECT face_down FROM cards WHERE cardid = (?1)",
            (self,),
            |row| row.get(0),
        )?;

        let mut vigilance = if face_down {
            false
        } else {
            db.query_row(
                "SELECT vigilance FROM cards WHERE cardid = (?1)",
                (self,),
                |row| row.get::<_, Option<bool>>(0),
            )?
            .unwrap_or_default()
        };

        let mut statement = db.prepare(indoc! {"
                SELECT add_vigilance, remove_vigilance, source, controller, restrictions, active_seq
                FROM modifiers
                WHERE active AND (
                    global
                    OR entire_battlefield
                )
                UNION
                SELECT add_vigilance, remove_vigilance, source, controller, restrictions, active_seq
                FROM modifiers, json_each(modifiers.modifying)
                WHERE active AND json_each.value = (?1)
                ORDER BY active_seq ASC
            "})?;

        let rows = statement.query_map((self,), |row| {
            Ok((
                row.get::<_, Option<bool>>(0)?,
                row.get::<_, Option<bool>>(1)?,
                row.get(2)?,
                serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
            ))
        })?;

        for row in rows {
            let (add, remove, source, controller_restriction, restrictions): (
                _,
                _,
                _,
                _,
                Vec<Restriction>,
            ) = row?;

            if self.passes_restrictions(
                db,
                source,
                source.controller(db)?,
                controller_restriction,
                &restrictions,
            )? {
                if add.unwrap_or_default() {
                    vigilance = true;
                }
                if remove.unwrap_or_default() {
                    vigilance = false;
                }
            }
        }

        Ok(vigilance)
    }

    pub fn location(self, db: &Connection) -> anyhow::Result<Location> {
        Ok(db.query_row(
            "SELECT location FROM cards WHERE cards.cardid = (?1)",
            (self,),
            |row| Ok(serde_json::from_str(&row.get::<_, String>(0)?).unwrap()),
        )?)
    }

    pub fn aura(self, db: &Connection) -> anyhow::Result<Option<AuraId>> {
        Ok(db.query_row(
            indoc! {"
                SELECT aura FROM cards WHERE cards.cardid = (?1)
            "},
            (if let Some(cloning) = self.cloning(db)? {
                cloning
            } else {
                self
            },),
            |row| row.get(0),
        )?)
    }

    pub fn colors(self, db: &Connection) -> anyhow::Result<HashSet<Color>> {
        let (mut colors, cost): (HashSet<Color>, CastingCost) = db.query_row(
            "SELECT colors, casting_cost FROM cards WHERE cardid = (?1)",
            (if let Some(cloning) = self.cloning(db)? {
                cloning
            } else {
                self
            },),
            |row| {
                Ok((
                    serde_json::from_str(&row.get::<_, String>(0)?).unwrap(),
                    serde_json::from_str(&row.get::<_, String>(1)?).unwrap(),
                ))
            },
        )?;

        colors.extend(cost.colors());

        let mut statement = db.prepare(indoc! {"
                SELECT color_modifiers, source, controller, restrictions, active_seq FROM modifiers
                WHERE active AND (
                    global
                    OR entire_battlefield
                ) AND color_modifiers IS NOT NULL
                UNION
                SELECT color_modifiers, source, controller, restrictions, active_seq
                FROM modifiers, json_each(modifiers.modifying)
                WHERE active AND json_each.value = (?1) AND color_modifiers IS NOT NULL
                ORDER BY active_seq ASC
            "})?;

        let rows = statement.query_map((self,), |row| {
            Ok((
                serde_json::from_str::<HashSet<Color>>(&row.get::<_, String>(0)?).unwrap(),
                row.get(1)?,
                serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                serde_json::from_str::<Vec<_>>(&row.get::<_, String>(3)?).unwrap(),
            ))
        })?;

        for row in rows {
            let (add_colors, source, controller_restriction, restrictions) = row?;

            if self.passes_restrictions(
                db,
                source,
                source.controller(db)?,
                controller_restriction,
                &restrictions,
            )? {
                colors.extend(add_colors);
            }
        }

        Ok(colors)
    }

    pub fn color_identity(self, db: &Connection) -> anyhow::Result<HashSet<Color>> {
        let mut identity = self.colors(db)?;

        for ability in self.activated_abilities(db)? {
            let ability = ability.ability(db)?;
            for mana in ability.cost.mana_cost {
                let color = mana.color();
                identity.insert(color);
            }

            for effect in ability.effects {
                match effect.into_effect(db, self.controller(db)?)? {
                    Effect::GainMana { mana } => match mana {
                        GainMana::Specific { gains } => {
                            for gain in gains.iter() {
                                identity.insert(gain.color());
                            }
                        }
                        GainMana::Choice { choices } => {
                            for choice in choices.iter() {
                                for mana in choice.iter() {
                                    identity.insert(mana.color());
                                }
                            }
                        }
                    },
                    _ => {}
                }
            }
        }

        Ok(identity)
    }

    pub fn types_intersect(self, db: &Connection, types: &HashSet<Type>) -> anyhow::Result<bool> {
        Ok(types.is_empty() || !self.types(db)?.is_disjoint(types))
    }

    pub fn types(self, db: &Connection) -> anyhow::Result<HashSet<Type>> {
        let mut types: HashSet<Type> = db.query_row(
            "SELECT types FROM cards WHERE cardid = (?1)",
            (if let Some(cloning) = self.cloning(db)? {
                cloning
            } else {
                self
            },),
            |row| Ok(serde_json::from_str(&row.get::<_, String>(0)?).unwrap()),
        )?;

        let mut statement = db.prepare(indoc! {"
                SELECT type_modifiers, source, controller, restrictions, active_seq FROM modifiers
                WHERE active AND (
                    global
                    OR entire_battlefield
                ) AND type_modifiers IS NOT NULL
                UNION
                SELECT type_modifiers, source, controller, restrictions, active_seq
                FROM modifiers, json_each(modifiers.modifying)
                WHERE active AND json_each.value = (?1) AND type_modifiers IS NOT NULL
                ORDER BY active_seq ASC
            "})?;

        let rows = statement.query_map((self,), |row| {
            Ok((
                serde_json::from_str::<HashSet<Type>>(&row.get::<_, String>(0)?).unwrap(),
                row.get(1)?,
                serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                serde_json::from_str::<Vec<_>>(&row.get::<_, String>(3)?).unwrap(),
            ))
        })?;

        for row in rows {
            let (add_types, source, controller_restriction, restrictions) = row?;

            if self.passes_restrictions_given_types(
                db,
                source,
                source.controller(db)?,
                controller_restriction,
                &restrictions,
                &types,
                &self.subtypes(db)?,
            )? {
                types.extend(add_types);
            }
        }

        Ok(types)
    }

    pub fn subtypes_intersect(
        self,
        db: &Connection,
        types: &HashSet<Subtype>,
    ) -> anyhow::Result<bool> {
        Ok(types.is_empty() || !self.subtypes(db)?.is_disjoint(types))
    }

    pub fn subtypes(self, db: &Connection) -> anyhow::Result<HashSet<Subtype>> {
        let face_down: bool = db.query_row(
            "SELECT face_down FROM cards WHERE cardid = (?1)",
            (self,),
            |row| row.get(0),
        )?;

        let mut types: HashSet<Subtype> = if face_down {
            Default::default()
        } else {
            db.query_row(
                "SELECT subtypes FROM cards WHERE cardid = (?1)",
                (if let Some(cloning) = self.cloning(db)? {
                    cloning
                } else {
                    self
                },),
                |row| Ok(serde_json::from_str(&row.get::<_, String>(0)?).unwrap()),
            )?
        };

        let mut statement = db.prepare(indoc! {"
                SELECT subtype_modifiers, remove_all_subtypes, source, controller, restrictions, active_seq
                FROM modifiers
                WHERE active AND (
                    global
                    OR entire_battlefield
                )
                UNION
                SELECT base_power_modifier, add_power_modifier, source, controller, restrictions, active_seq
                FROM modifiers, json_each(modifiers.modifying)
                WHERE active AND json_each.value = (?1)
                ORDER BY active_seq ASC
            "})?;

        let rows = statement.query_map((self,), |row| {
            Ok((
                row.get::<_, Option<String>>(0)?
                    .map(|row| serde_json::from_str::<HashSet<Subtype>>(&row).unwrap()),
                row.get::<_, Option<bool>>(1)?,
                row.get(2)?,
                serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                serde_json::from_str::<Vec<_>>(&row.get::<_, String>(4)?).unwrap(),
            ))
        })?;

        for row in rows {
            let (add_types, remove_all, source, controller_restriction, restrictions) = row?;

            if self.passes_restrictions_given_types(
                db,
                source,
                source.controller(db)?,
                controller_restriction,
                &restrictions,
                &self.types(db)?,
                &types,
            )? {
                if remove_all.unwrap_or_default() {
                    types.clear();
                }

                if let Some(add_types) = add_types {
                    types.extend(add_types);
                }
            }
        }

        Ok(types)
    }

    pub fn upload(
        db: &Connection,
        cards: &Cards,
        player: PlayerId,
        name: &str,
    ) -> anyhow::Result<CardId> {
        let card = cards.get(name).expect("Valid card name");

        Self::upload_card(db, card, player, Location::Library, false)
    }

    pub fn upload_token(db: &Connection, player: PlayerId, token: Token) -> anyhow::Result<CardId> {
        let card: Card = token.into();

        Self::upload_card(db, &card, player, Location::Battlefield, true)
    }

    fn upload_card(
        db: &Connection,
        card: &Card,
        player: PlayerId,
        destination: Location,
        is_token: bool,
    ) -> anyhow::Result<CardId> {
        let cardid = CardId::new();
        db.execute(
            UPLOAD_CARD_SQL,
            (
                cardid,
                0,
                serde_json::to_string(&destination)?,
                card.name.clone(),
                player,
                player,
                false,
                false,
                false,
                is_token,
                card.cannot_be_countered,
                card.split_second,
            ),
        )?;

        db.execute(
            indoc! {"
            UPDATE cards
            SET casting_cost = (?2),
                power = (?3),
                toughness = (?4),
                types = (?5),
                subtypes = (?6),
                colors = (?7)
            WHERE cards.cardid = (?1)
        "},
            (
                cardid,
                serde_json::to_string(&card.cost)?,
                card.power,
                card.toughness,
                serde_json::to_string(&card.types)?,
                serde_json::to_string(&card.subtypes)?,
                serde_json::to_string(&card.colors)?,
            ),
        )?;

        if let Some(enchant) = &card.enchant {
            let mut statement = db.prepare(UPLOAD_MODIFIER_SQL)?;

            let mut modifierids = vec![];
            for modifier in enchant.modifiers.iter() {
                let modifierid = upload_modifier(&mut statement, cardid, modifier, db, false)?;
                modifierids.push(modifierid);
            }

            let auraid = AuraId::new();
            db.execute(
                indoc! {"
                    INSERT INTO auras (auraid, modifiers, restrictions) VALUES (?1, ?2, ?3)
                "},
                (
                    auraid,
                    serde_json::to_string(&modifierids)?,
                    serde_json::to_string(&enchant.restrictions)?,
                ),
            )?;

            db.execute(
                indoc! {"
                    UPDATE cards
                    SET aura = (?2)
                    WHERE cards.cardid = (?1)
                "},
                (cardid, auraid),
            )?;
        }

        if !card.effects.is_empty() {
            db.execute(
                indoc! {"
                    UPDATE cards
                    SET effects = (?2)
                    WHERE cards.cardid = (?1)
                "},
                (cardid, serde_json::to_string(&card.effects)?),
            )?;
        }

        if !card.etb_abilities.is_empty() {
            db.execute(
                indoc! {"
                    UPDATE cards
                    SET etb = (?2)
                    WHERE cards.cardid = (?1)
                "},
                (cardid, serde_json::to_string(&card.etb_abilities)?),
            )?;
        }

        if !card.static_abilities.is_empty() {
            db.execute(
                indoc! {"
                    UPDATE cards
                    SET abilities = (?2)
                    WHERE cards.cardid = (?1)
                "},
                (cardid, serde_json::to_string(&card.static_abilities)?),
            )?;
        }

        if !card.activated_abilities.is_empty() {
            let mut ability_ids = vec![];
            for ability in card.activated_abilities.iter() {
                let id = AbilityId::upload_ability(db, cardid, ability)?;

                ability_ids.push(id);
            }

            db.execute(
                indoc! {"
                    UPDATE cards
                    SET activated_abilities = (?2)
                    WHERE cards.cardid = (?1)
                "},
                (cardid, serde_json::to_string(&ability_ids)?),
            )?;
        }

        if !card.triggered_abilities.is_empty() {
            let mut trigger_ids = vec![];
            for ability in card.triggered_abilities.iter() {
                let triggerid = TriggerId::new();
                trigger_ids.push(triggerid);

                db.execute(
                    indoc! {"
                        INSERT INTO triggers (
                            triggerid,
                            listener,
                            source,
                            location_from,
                            for_types,
                            effects,
                            active,
                            in_stack
                        ) VALUES (
                            (?1),
                            (?2),
                            (?3),
                            (?4),
                            (?5),
                            (?6),
                            (?7),
                            (?8)
                        )"},
                    (
                        triggerid,
                        cardid,
                        serde_json::to_string(&ability.trigger.trigger)?,
                        serde_json::to_string(&ability.trigger.from)?,
                        serde_json::to_string(&ability.trigger.for_types)?,
                        serde_json::to_string(&ability.effects)?,
                        false,
                        false,
                    ),
                )?;
            }

            db.execute(
                "UPDATE cards SET triggered_abilities = (?2) WHERE cards.cardid = (?1)",
                (cardid, serde_json::to_string(&trigger_ids)?),
            )?;
        }

        Ok(cardid)
    }

    pub fn cost(self, db: &Connection) -> anyhow::Result<CastingCost> {
        Ok(db.query_row(
            "SELECT casting_cost FROM cards WHERE cardid = (?1)",
            (self,),
            |row| Ok(serde_json::from_str(&row.get::<_, String>(0)?).unwrap()),
        )?)
    }

    pub fn valid_targets(self, db: &Connection) -> anyhow::Result<HashSet<ActiveTarget>> {
        let mut targets = HashSet::default();
        let creatures = Battlefield::creatures(db)?;

        for effect in self.effects(db)? {
            let effect = effect.into_effect(db, self.controller(db)?)?;

            match effect {
                Effect::CounterSpell { target } => {
                    targets_for_counterspell(db, self.controller(db)?, target, &mut targets)?;
                }
                Effect::GainMana { .. } => {}
                Effect::BattlefieldModifier(_) => {}
                Effect::ControllerDrawCards(_) => {}
                Effect::ModifyCreature(modifier) => {
                    targets_for_battlefield_modifier(
                        db,
                        self,
                        Some(&modifier),
                        &creatures,
                        self.controller(db)?,
                        &mut targets,
                    )?;
                }
                Effect::ExileTargetCreature => {
                    for creature in creatures.iter() {
                        if creature.can_be_targeted(db, self.controller(db)?)? {
                            targets.insert(ActiveTarget::Battlefield { id: *creature });
                        }
                    }
                }
                Effect::ExileTargetCreatureManifestTopOfLibrary => {
                    for creature in creatures.iter() {
                        if creature.can_be_targeted(db, self.controller(db)?)? {
                            targets.insert(ActiveTarget::Battlefield { id: *creature });
                        }
                    }
                }
                Effect::DealDamage(dmg) => {
                    self.targets_for_damage(&creatures, db, dmg, &mut targets)?;
                }
                Effect::CreateToken(_) => {}
                Effect::Equip(_) => {
                    targets_for_battlefield_modifier(
                        db,
                        self,
                        None,
                        &creatures,
                        self.controller(db)?,
                        &mut targets,
                    )?;
                }
                Effect::GainCounter(_) => {}
            }
        }

        for ability in self.activated_abilities(db)? {
            self.targets_for_ability(db, ability, &creatures, &mut targets)?;
        }

        Ok(targets)
    }

    fn targets_for_damage(
        self,
        creatures: &[CardId],
        db: &Connection,
        dmg: DealDamage,
        targets: &mut HashSet<ActiveTarget>,
    ) -> anyhow::Result<()> {
        for creature in creatures.iter() {
            let controller = self.controller(db)?;
            if creature.can_be_targeted(db, controller)?
                && creature.passes_restrictions(
                    db,
                    self,
                    controller,
                    Controller::Any,
                    &dmg.restrictions,
                )?
            {
                targets.insert(ActiveTarget::Battlefield { id: *creature });
            }
        }

        Ok(())
    }

    pub fn targets_for_ability(
        self,
        db: &Connection,
        ability: AbilityId,
        creatures: &[CardId],
        targets: &mut HashSet<ActiveTarget>,
    ) -> Result<(), anyhow::Error> {
        let ability = ability.ability(db)?;
        if !ability.apply_to_self {
            for effect in ability.effects {
                let controller = self.controller(db)?;
                match effect.into_effect(db, controller)? {
                    Effect::CounterSpell { target } => {
                        targets_for_counterspell(db, controller, target, targets)?;
                    }
                    Effect::GainMana { .. } => {}
                    Effect::BattlefieldModifier(_) => {}
                    Effect::ControllerDrawCards(_) => {}
                    Effect::Equip(_) => {
                        targets_for_battlefield_modifier(
                            db,
                            self,
                            None,
                            creatures,
                            self.controller(db)?,
                            targets,
                        )?;
                    }
                    Effect::CreateToken(_) => todo!(),
                    Effect::DealDamage(dmg) => {
                        self.targets_for_damage(creatures, db, dmg, targets)?;
                    }
                    Effect::ExileTargetCreature => {
                        for creature in creatures.iter() {
                            if creature.can_be_targeted(db, self.controller(db)?)? {
                                targets.insert(ActiveTarget::Battlefield { id: *creature });
                            }
                        }
                    }
                    Effect::ExileTargetCreatureManifestTopOfLibrary => {
                        for creature in creatures.iter() {
                            if creature.can_be_targeted(db, self.controller(db)?)? {
                                targets.insert(ActiveTarget::Battlefield { id: *creature });
                            }
                        }
                    }
                    Effect::GainCounter(_) => {}
                    Effect::ModifyCreature(modifier) => {
                        targets_for_battlefield_modifier(
                            db,
                            self,
                            Some(&modifier),
                            creatures,
                            self.controller(db)?,
                            targets,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn can_be_countered(
        self,
        db: &Connection,
        caster: PlayerId,
        target: &SpellTarget,
    ) -> anyhow::Result<bool> {
        if db.query_row(
            "SELECT cannot_be_countered FROM cards WHERE cardid = (?1)",
            (self,),
            |row| row.get(0),
        )? {
            return Ok(false);
        }

        let SpellTarget {
            controller,
            types,
            subtypes,
        } = target;

        match controller {
            Controller::You => {
                if caster != self.controller(db)? {
                    return Ok(false);
                }
            }
            Controller::Opponent => {
                if caster == self.controller(db)? {
                    return Ok(false);
                }
            }
            Controller::Any => {}
        };

        if !types.is_empty() && !self.types_intersect(db, types)? {
            return Ok(false);
        }

        if !self.subtypes_intersect(db, subtypes)? {
            return Ok(false);
        }

        for (ability, ability_controller) in Battlefield::static_abilities(db)? {
            match &ability {
                StaticAbility::GreenCannotBeCountered { controller } => {
                    if self.colors(db)?.contains(&Color::Green) {
                        match controller {
                            Controller::You => {
                                if ability_controller == self.controller(db)? {
                                    return Ok(false);
                                }
                            }
                            Controller::Opponent => {
                                if ability_controller != self.controller(db)? {
                                    return Ok(false);
                                }
                            }
                            Controller::Any => {
                                return Ok(false);
                            }
                        }
                    }
                }
                StaticAbility::BattlefieldModifier(_) => {}
                StaticAbility::ExtraLandsPerTurn(_) => {}
            }
        }

        Ok(true)
    }

    pub fn can_be_targeted(self, db: &Connection, caster: PlayerId) -> anyhow::Result<bool> {
        if self.shroud(db)? {
            return Ok(false);
        }

        if self.hexproof(db)? && self.controller(db)? != caster {
            return Ok(false);
        }

        // TODO protection

        Ok(true)
    }

    pub fn can_be_sacrificed(self, _db: &Connection) -> anyhow::Result<bool> {
        Ok(true)
    }

    pub fn shroud(self, db: &Connection) -> anyhow::Result<bool> {
        let mut has_shroud = db
            .query_row(
                "SELECT shroud FROM cards WHERE cardid = (?1)",
                (self,),
                |row| row.get::<_, Option<bool>>(0),
            )?
            .unwrap_or_default();

        let mut statement = db.prepare(indoc! {"
                SELECT add_shroud, remove_shroud
                FROM modifiers, json_each(modifiers.modifying)
                WHERE active AND (
                    json_each.value = (?1)
                    OR global
                    OR entire_battlefield
                )
                ORDER BY active_seq ASC
            "})?;

        let rows = statement.query_map((self,), |row| {
            Ok((
                row.get::<_, Option<bool>>(0)?.unwrap_or_default(),
                row.get::<_, Option<bool>>(1)?.unwrap_or_default(),
            ))
        })?;

        for row in rows {
            let (add_shroud, remove_shroud) = row?;
            if add_shroud {
                has_shroud = true;
            }
            if remove_shroud {
                has_shroud = false;
            }
        }

        Ok(has_shroud)
    }

    pub fn hexproof(self, db: &Connection) -> anyhow::Result<bool> {
        let mut has_hexproof = db
            .query_row(
                "SELECT hexproof FROM cards WHERE cardid = (?1)",
                (self,),
                |row| row.get::<_, Option<bool>>(0),
            )?
            .unwrap_or_default();

        let mut statement = db.prepare(indoc! {"
                SELECT add_hexproof, remove_hexproof
                FROM modifiers, json_each(modifiers.modifying)
                WHERE active AND (
                    json_each.value = (?1)
                    OR global
                    OR entire_battlefield
                )
                ORDER BY active_seq ASC
            "})?;

        let rows = statement.query_map((self,), |row| {
            Ok((
                row.get::<_, Option<bool>>(0)?.unwrap_or_default(),
                row.get::<_, Option<bool>>(1)?.unwrap_or_default(),
            ))
        })?;

        for row in rows {
            let (add_hexproof, remove_hexproof) = row?;
            if add_hexproof {
                has_hexproof = true;
            }
            if remove_hexproof {
                has_hexproof = false;
            }
        }

        Ok(has_hexproof)
    }

    pub fn tapped(self, db: &Connection) -> anyhow::Result<bool> {
        Ok(db.query_row(
            "SELECT tapped FROM cards WHERE cardid = (?1)",
            (self,),
            |row| row.get(0),
        )?)
    }

    pub fn tap(self, db: &Connection) -> anyhow::Result<()> {
        db.execute(
            "UPDATE cards SET tapped = TRUE WHERE cardid = (?1)",
            (self,),
        )?;

        Ok(())
    }

    pub fn clone_card(&self, db: &Connection, source: CardId) -> anyhow::Result<()> {
        db.execute(
            "UPDATE cards SET cloning = (?2) WHERE cardid = (?1)",
            (self, source),
        )?;

        Ok(())
    }

    pub fn cloning(self, db: &Connection) -> anyhow::Result<Option<CardId>> {
        Ok(db.query_row(
            "SELECT cloning FROM cards WHERE cardid = (?1)",
            (self,),
            |row| row.get(0),
        )?)
    }

    pub fn is_land(self, db: &Connection) -> anyhow::Result<bool> {
        self.types_intersect(db, &HashSet::from([Type::Land, Type::BasicLand]))
    }

    pub(crate) fn manifest(self, db: &Connection) -> anyhow::Result<()> {
        db.execute(
            "UPDATE cards SET manifested = TRUE, face_down = TRUE WHERE cardid = (?1)",
            (self,),
        )?;

        Ok(())
    }

    pub fn is_permanent(self, db: &Connection) -> anyhow::Result<bool> {
        Ok(!self.types_intersect(db, &HashSet::from([Type::Instant, Type::Sorcery]))?)
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Copy, Hash, Default)]
pub struct AuraId(usize);

impl AuraId {
    pub fn modifiers(self, db: &Connection) -> anyhow::Result<Vec<ModifierId>> {
        Ok(db.query_row(
            indoc! {"
                    SELECT modifiers FROM auras WHERE auraid = (?1)
                "},
            (self,),
            |row| Ok(serde_json::from_str(&row.get::<_, String>(0)?).unwrap()),
        )?)
    }

    pub fn is_attached(self, db: &Connection) -> anyhow::Result<bool> {
        let modifiers = self.modifiers(db)?;
        for modifier in modifiers {
            if !modifier.modifying(db)?.is_empty() {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

impl ToSql for AuraId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

impl FromSql for AuraId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(Self(usize::column_result(value)?))
    }
}

impl AuraId {
    pub fn new() -> Self {
        Self(NEXT_AURA_ID.fetch_add(1, Ordering::Relaxed))
    }
}

fn upload_modifier(
    statement: &mut rusqlite::Statement<'_>,
    source: CardId,
    modifier: &BattlefieldModifier,
    db: &Connection,
    temporary: bool,
) -> Result<ModifierId, anyhow::Error> {
    let modifierid = ModifierId::new();

    statement.execute((
        modifierid,
        serde_json::to_string(&modifier.duration)?,
        temporary,
        serde_json::to_string(&modifier.controller)?,
        serde_json::to_string(&modifier.restrictions)?,
        modifier.modifier.global,
        modifier.modifier.entire_battlefield,
        false,
    ))?;

    db.execute(
        "UPDATE modifiers SET source = (?2) WHERE modifierid = (?1)",
        (modifierid, source),
    )?;

    if let Some(power) = modifier.modifier.base_power {
        db.execute(
            indoc! {"
                UPDATE modifiers
                SET base_power_modifier = (?2)
                WHERE modifiers.modifierid = (?1)
            "},
            (modifierid, power),
        )?;
    }
    if let Some(toughness) = modifier.modifier.base_toughness {
        db.execute(
            indoc! {"
                UPDATE modifiers
                SET base_toughness_modifier = (?2)
                WHERE modifiers.modifierid = (?1)
            "},
            (modifierid, toughness),
        )?;
    }
    if let Some(dynamic) = &modifier.modifier.dynamic_power_toughness {
        db.execute(
            indoc! {"
                UPDATE modifiers
                SET dynamic_add_power_toughness = (?2)
                WHERE modifiers.modifierid = (?1)
            "},
            (modifierid, serde_json::to_string(dynamic)?),
        )?;
    }
    if let Some(power) = modifier.modifier.add_power {
        db.execute(
            indoc! {"
                UPDATE modifiers
                SET add_power_modifier = (?2)
                WHERE modifiers.modifierid = (?1)
            "},
            (modifierid, power),
        )?;
    }
    if let Some(toughness) = modifier.modifier.add_toughness {
        db.execute(
            indoc! {"
                UPDATE modifiers
                SET add_toughness_modifier = (?2)
                WHERE modifiers.modifierid = (?1)
            "},
            (modifierid, toughness),
        )?;
    }
    if !modifier.modifier.add_types.is_empty() {
        db.execute(
            indoc! {"
                UPDATE modifiers
                SET type_modifiers = (?2)
                WHERE modifiers.modifierid = (?1)
            "},
            (
                modifierid,
                serde_json::to_string(&modifier.modifier.add_types)?,
            ),
        )?;
    }
    if !modifier.modifier.add_subtypes.is_empty() {
        db.execute(
            indoc! {"
                UPDATE modifiers
                SET subtype_modifiers = (?2)
                WHERE modifiers.modifierid = (?1)
            "},
            (
                modifierid,
                serde_json::to_string(&modifier.modifier.add_subtypes)?,
            ),
        )?;
    }
    if modifier.modifier.remove_all_subtypes {
        db.execute(
            indoc! {"
                UPDATE modifiers
                SET remove_all_subtypes = (?2)
                WHERE modifiers.modifierid = (?1)
            "},
            (modifierid, true),
        )?;
    }
    if modifier.modifier.remove_all_abilities {
        modifierid.remove_all_abilities(db)?;
    }

    if modifier.modifier.add_vigilance {
        db.execute(
            indoc! {"
                UPDATE modifiers
                SET add_vigilance = (?2)
                WHERE modifiers.modifierid = (?1)
            "},
            (modifierid, true),
        )?;
    }

    if let Some(ability) = &modifier.modifier.add_ability {
        db.execute(
            indoc! {"
                UPDATE modifiers
                SET activated_ability_modifier = (?2)
                WHERE modifiers.modifierid = (?1)
            "},
            (
                modifierid,
                serde_json::to_string(&ActivatedAbilityModifier::Add(AbilityId::upload_ability(
                    db, source, ability,
                )?))?,
            ),
        )?;
    }

    Ok(modifierid)
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, Default, From)]
pub struct AbilityId(usize);

impl ToSql for AbilityId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

impl FromSql for AbilityId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(Self(usize::column_result(value)?))
    }
}

impl AbilityId {
    pub fn new() -> Self {
        Self(NEXT_ABILITY_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn upload_ability(
        db: &Connection,
        cardid: CardId,
        ability: &ActivatedAbility,
    ) -> anyhow::Result<AbilityId> {
        let id = AbilityId::new();
        db.execute(
            INSERT_ABILITIES_SQL,
            (
                id,
                cardid,
                ability.apply_to_self,
                serde_json::to_string(&ability.cost)?,
                serde_json::to_string(&ability.effects)?,
                false,
            ),
        )?;

        Ok(id)
    }

    pub fn land_abilities(db: &Connection) -> HashMap<Subtype, Self> {
        INIT_LAND_ABILITIES.with(|init| {
            init.get_or_init(|| {
                let mut abilities = HashMap::new();

                let id = AbilityId::new();
                db.execute(
                    INSERT_ABILITIES_SQL,
                    (
                        id,
                        Option::<CardId>::None,
                        false,
                        serde_json::to_string(&AbilityCost {
                            mana_cost: vec![],
                            tap: true,
                            additional_cost: vec![],
                        })
                        .unwrap(),
                        serde_json::to_string(&[AnyEffect {
                            effect: Effect::GainMana {
                                mana: GainMana::Specific {
                                    gains: vec![Mana::White],
                                },
                            },
                            threshold: None,
                        }])
                        .unwrap(),
                        false,
                    ),
                )
                .unwrap();

                abilities.insert(Subtype::Plains, id);

                let id = AbilityId::new();
                db.execute(
                    INSERT_ABILITIES_SQL,
                    (
                        id,
                        Option::<CardId>::None,
                        false,
                        serde_json::to_string(&AbilityCost {
                            mana_cost: vec![],
                            tap: true,
                            additional_cost: vec![],
                        })
                        .unwrap(),
                        serde_json::to_string(&[AnyEffect {
                            effect: Effect::GainMana {
                                mana: GainMana::Specific {
                                    gains: vec![Mana::Blue],
                                },
                            },
                            threshold: None,
                        }])
                        .unwrap(),
                        false,
                    ),
                )
                .unwrap();

                abilities.insert(Subtype::Island, id);

                let id = AbilityId::new();
                db.execute(
                    INSERT_ABILITIES_SQL,
                    (
                        id,
                        Option::<CardId>::None,
                        false,
                        serde_json::to_string(&AbilityCost {
                            mana_cost: vec![],
                            tap: true,
                            additional_cost: vec![],
                        })
                        .unwrap(),
                        serde_json::to_string(&[AnyEffect {
                            effect: Effect::GainMana {
                                mana: GainMana::Specific {
                                    gains: vec![Mana::Black],
                                },
                            },
                            threshold: None,
                        }])
                        .unwrap(),
                        false,
                    ),
                )
                .unwrap();

                abilities.insert(Subtype::Swamp, id);

                let id = AbilityId::new();
                db.execute(
                    INSERT_ABILITIES_SQL,
                    (
                        id,
                        Option::<CardId>::None,
                        false,
                        serde_json::to_string(&AbilityCost {
                            mana_cost: vec![],
                            tap: true,
                            additional_cost: vec![],
                        })
                        .unwrap(),
                        serde_json::to_string(&[AnyEffect {
                            effect: Effect::GainMana {
                                mana: GainMana::Specific {
                                    gains: vec![Mana::Red],
                                },
                            },
                            threshold: None,
                        }])
                        .unwrap(),
                        false,
                    ),
                )
                .unwrap();

                abilities.insert(Subtype::Mountain, id);

                let id = AbilityId::new();
                db.execute(
                    INSERT_ABILITIES_SQL,
                    (
                        id,
                        Option::<CardId>::None,
                        false,
                        serde_json::to_string(&AbilityCost {
                            mana_cost: vec![],
                            tap: true,
                            additional_cost: vec![],
                        })
                        .unwrap(),
                        serde_json::to_string(&[AnyEffect {
                            effect: Effect::GainMana {
                                mana: GainMana::Specific {
                                    gains: vec![Mana::Green],
                                },
                            },
                            threshold: None,
                        }])
                        .unwrap(),
                        false,
                    ),
                )
                .unwrap();

                abilities.insert(Subtype::Forest, id);

                abilities
            })
            .clone()
        })
    }

    pub fn move_to_stack(
        self,
        db: &Connection,
        source: CardId,
        targets: HashSet<ActiveTarget>,
    ) -> anyhow::Result<()> {
        if Stack::split_second(db)? {
            return Ok(());
        }

        db.execute(
            indoc! {"
                UPDATE abilities
                SET in_stack = TRUE,
                    stack_seq = (?2),
                    targets = (?3),
                    source = (?4)
                WHERE abilities.abilityid = (?1)
            "},
            (
                self,
                NEXT_STACK_SEQ.fetch_add(1, Ordering::Relaxed),
                serde_json::to_string(&targets)?,
                // This is a hack to make land types work, probably need something better here
                source,
            ),
        )?;

        Ok(())
    }

    pub fn ability(self, db: &Connection) -> anyhow::Result<ActivatedAbility> {
        Ok(db.query_row(
            "SELECT cost, effects, apply_to_self FROM abilities WHERE abilityid = (?1)",
            (self,),
            |row| {
                Ok(ActivatedAbility {
                    cost: serde_json::from_str(&row.get::<_, String>(0)?).unwrap(),
                    effects: serde_json::from_str(&row.get::<_, String>(1)?).unwrap(),
                    apply_to_self: row.get(2)?,
                })
            },
        )?)
    }

    pub fn apply_to_self(self, db: &Connection) -> anyhow::Result<bool> {
        Ok(db.query_row(
            "SELECT  apply_to_self FROM abilities WHERE abilityid = (?1)",
            (self,),
            |row| row.get(0),
        )?)
    }

    pub fn effects(self, db: &Connection) -> anyhow::Result<Vec<AnyEffect>> {
        Ok(db.query_row(
            "SELECT effects FROM abilities WHERE abilityid = (?1)",
            (self,),
            |row| Ok(serde_json::from_str(&row.get::<_, String>(0)?).unwrap()),
        )?)
    }

    pub fn source(self, db: &Connection) -> anyhow::Result<CardId> {
        Ok(db.query_row(
            "SELECT source FROM abilities WHERE abilityid = (?1)",
            (self,),
            |row| row.get(0),
        )?)
    }

    pub(crate) fn controller(self, db: &Connection) -> anyhow::Result<PlayerId> {
        self.source(db)?.controller(db)
    }

    fn delete(self, db: &Connection) -> anyhow::Result<()> {
        db.execute("DELETE FROM abilities WHERE abilityid = (?1)", (self,))?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, Default)]
pub struct ModifierId(usize);

impl ToSql for ModifierId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

impl FromSql for ModifierId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(Self(usize::column_result(value)?))
    }
}

impl ModifierId {
    pub fn new() -> Self {
        Self(NEXT_MODIFIER_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn remove_all_abilities(self, db: &Connection) -> Result<(), anyhow::Error> {
        db.execute(
            indoc! {"
                UPDATE modifiers
                SET activated_ability_modifier = (?2),
                    static_ability_modifier = (?3),
                    triggered_ability_modifier = (?4)
                WHERE modifiers.modifierid = (?1)
            "},
            (
                self,
                serde_json::to_string(&ActivatedAbilityModifier::RemoveAll)?,
                serde_json::to_string(&StaticAbilityModifier::RemoveAll)?,
                serde_json::to_string(&TriggeredAbilityModifier::RemoveAll)?,
            ),
        )?;
        db.execute(
            indoc! {"
                UPDATE modifiers
                SET remove_vigilance = TRUE,
                    remove_flash = TRUE,
                    remove_hexproof = TRUE,
                    remove_shroud = TRUE
                WHERE modifiers.modifierid = (?1)
            "},
            (self,),
        )?;

        Ok(())
    }

    pub fn upload_single_modifier(
        db: &Connection,
        cardid: CardId,
        modifier: &BattlefieldModifier,
        temporary: bool,
    ) -> anyhow::Result<ModifierId> {
        let mut statement = db.prepare(UPLOAD_MODIFIER_SQL)?;
        upload_modifier(&mut statement, cardid, modifier, db, temporary)
    }

    pub fn modifying(self, db: &Connection) -> anyhow::Result<Vec<CardId>> {
        Ok(db.query_row(
            indoc! {"
                    SELECT modifying FROM modifiers WHERE modifierid = (?1)
                "},
            (self,),
            |row| {
                Ok(row
                    .get::<_, Option<String>>(0)?
                    .as_ref()
                    .map(|s| serde_json::from_str(s).unwrap())
                    .unwrap_or_default())
            },
        )?)
    }

    pub fn ability_modifier(
        self,
        db: &Connection,
    ) -> anyhow::Result<Option<ActivatedAbilityModifier>> {
        Ok(db.query_row(
            indoc! {"
                    SELECT modifying FROM modifiers WHERE modifierid = (?1)
                "},
            (self,),
            |row| {
                Ok(row
                    .get::<_, Option<String>>(0)?
                    .as_ref()
                    .map(|s| serde_json::from_str(s).unwrap())
                    .unwrap_or_default())
            },
        )?)
    }

    pub fn active_modifiers(db: &Connection) -> anyhow::Result<Vec<ModifierId>> {
        let mut statement = db.prepare(indoc! {"
            SELECT modifierid FROM modifiers WHERE modifiers.active
        "})?;

        let rows = statement.query_map((), |row| row.get(0))?;
        rows.into_iter()
            .map(|v| Ok(v?))
            .collect::<anyhow::Result<_>>()
    }

    pub fn activate(self, db: &Connection) -> anyhow::Result<()> {
        db.execute(
            indoc! {"
                UPDATE modifiers
                SET active = TRUE
                WHERE modifierid = (?1)
            "},
            (self,),
        )?;
        db.execute(
            indoc! {"
                UPDATE modifiers
                SET active_seq = (?2)
                WHERE modifierid = (?1) AND active_seq IS NULL
            "},
            (self, NEXT_MODIFIER_SEQ.fetch_add(1, Ordering::Relaxed)),
        )?;

        Ok(())
    }

    pub fn deactivate(self, db: &Connection) -> anyhow::Result<()> {
        let (is_temporary, modifying) = db.query_row(
            "SELECT is_temporary, modifying FROM modifiers WHERE modifierid = (?1)",
            (self,),
            |row| {
                Ok((
                    row.get::<_, Option<bool>>(0)?,
                    row.get::<_, Option<String>>(1)?
                        .as_ref()
                        .map(|s| serde_json::from_str::<Vec<CardId>>(s).unwrap())
                        .unwrap_or_default(),
                ))
            },
        )?;

        if is_temporary.unwrap_or_default() && modifying.is_empty() {
            if let Some(ActivatedAbilityModifier::Add(ability)) = self.ability_modifier(db)? {
                ability.delete(db)?;
            }

            db.execute("DELETE FROM modifiers WHERE modifierid = (?1)", (self,))?;
        } else {
            db.execute(
                indoc! {"
                    UPDATE modifiers
                    SET active = FALSE
                    WHERE modifiers.modifierid = (?1)
                "},
                (self,),
            )?;
        }

        Ok(())
    }

    pub fn detach_all(&self, db: &Connection) -> anyhow::Result<()> {
        db.execute(
            "UPDATE modifiers SET modifying = NULL WHERE modifierid = (?1)",
            (self,),
        )?;
        self.deactivate(db)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, Default, From)]
pub struct TriggerId(usize);

impl ToSql for TriggerId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

impl FromSql for TriggerId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(Self(usize::column_result(value)?))
    }
}

impl TriggerId {
    pub fn new() -> Self {
        Self(NEXT_TRIGGER_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn move_to_stack(
        self,
        db: &Connection,
        targets: HashSet<ActiveTarget>,
    ) -> anyhow::Result<()> {
        if Stack::split_second(db)? {
            return Ok(());
        }

        db.execute(
            indoc! {"
                UPDATE triggers 
                SET in_stack = TRUE,
                    stack_seq = (?2),
                    targets = (?3)
                WHERE triggers.triggerid = (?1)
            "},
            (
                self,
                NEXT_STACK_SEQ.fetch_add(1, Ordering::Relaxed),
                serde_json::to_string(&targets)?,
            ),
        )?;

        Ok(())
    }

    pub fn location_from(self, db: &Connection) -> anyhow::Result<triggers::Location> {
        Ok(db.query_row(
            "SELECT location_from FROM triggers WHERE triggerid = (?1)",
            (self,),
            |row| Ok(serde_json::from_str(&row.get::<_, String>(0)?).unwrap()),
        )?)
    }

    pub fn for_types(self, db: &Connection) -> anyhow::Result<HashSet<Type>> {
        Ok(db.query_row(
            "SELECT for_types FROM triggers WHERE triggerid = (?1)",
            (self,),
            |row| Ok(serde_json::from_str(&row.get::<_, String>(0)?).unwrap()),
        )?)
    }

    pub fn listener(self, db: &Connection) -> anyhow::Result<CardId> {
        Ok(db.query_row(
            "SELECT listener FROM triggers WHERE triggerid = (?1)",
            (self,),
            |row| row.get(0),
        )?)
    }

    pub fn triggered_ability(self, db: &Connection) -> anyhow::Result<TriggeredAbility> {
        Ok(db.query_row(
            indoc! {"
                    SELECT source, location_from, for_types, effects FROM triggers WHERE triggerid = (?1)
                "},
            (self,),
            |row| {
                Ok(TriggeredAbility {
                    trigger: Trigger {
                        trigger: serde_json::from_str(&row.get::<_, String>(0)?).unwrap(),
                        from: serde_json::from_str(&row.get::<_, String>(1)?).unwrap(),
                        for_types: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                    },
                    effects: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                })
            },
        )?)
    }

    pub fn effects(self, db: &Connection) -> anyhow::Result<Vec<AnyEffect>> {
        Ok(db.query_row(
            "SELECT effects FROM triggers WHERE triggerid = (?1)",
            (self,),
            |row| Ok(serde_json::from_str(&row.get::<_, String>(0)?).unwrap()),
        )?)
    }

    pub fn active_triggers_of_type(
        db: &Connection,
        trigger: TriggerSource,
    ) -> anyhow::Result<Vec<TriggerId>> {
        let mut results = vec![];
        let mut of_type = db.prepare("SELECT triggerid FROM triggers WHERE source = (?1)")?;
        for row in of_type.query_map((serde_json::to_string(&trigger)?,), |row| row.get(0))? {
            results.push(row?);
        }

        Ok(results)
    }

    pub fn activate_all_for_card(db: &Connection, cardid: CardId) -> anyhow::Result<()> {
        db.execute(
            indoc! {"
                UPDATE triggers
                SET active = TRUE
                WHERE listener = (?1)
            "},
            (cardid,),
        )?;

        Ok(())
    }

    pub fn deactivate_all_for_card(db: &Connection, cardid: CardId) -> anyhow::Result<()> {
        db.execute(
            indoc! {"
                UPDATE triggers
                SET active = FALSE
                WHERE listener = (?1)
            "},
            (cardid,),
        )?;

        Ok(())
    }

    pub fn activate(self, db: &Connection) -> anyhow::Result<()> {
        db.execute(
            indoc! {"
                UPDATE triggers
                SET active = TRUE
                WHERE triggerid = (?1)
            "},
            (self,),
        )?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash, Default)]
pub struct CounterId(usize);

impl CounterId {
    pub fn new() -> Self {
        Self(NEXT_COUNTER_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn add_counters(
        db: &Connection,
        card: CardId,
        counter: Counter,
        count: usize,
    ) -> anyhow::Result<()> {
        let mut existing = db.prepare(indoc! {"
                SELECT counterid, count
                FROM counters
                WHERE is_on = (?1)
                    AND type = (?2)
        "})?;

        let mut rows = existing.query_map((card, serde_json::to_string(&counter)?), |row| {
            Ok((row.get::<_, CounterId>(0)?, row.get::<_, usize>(1)?))
        })?;

        if let Some((id, existing_count)) = rows.next().map_or(Ok(None), |v| v.map(Some))? {
            db.execute(
                "UPDATE counters SET count = (?2) WHERE counterid = (?1)",
                (id, existing_count + count),
            )?;
        } else {
            db.execute(
                indoc! {"
                    INSERT INTO counters (
                        is_on, type, count
                    ) VALUES (
                        (?1), (?2), (?3)
                    )
                "},
                (card, serde_json::to_string(&counter)?, count),
            )?;
        }

        Ok(())
    }

    pub fn counters_on(db: &Connection, card: CardId, counter: Counter) -> anyhow::Result<usize> {
        let mut existing = db.prepare(indoc! {"
                SELECT count
                FROM counters
                WHERE is_on = (?1)
                    AND type = (?2)
        "})?;

        let mut rows = existing.query_map((card, serde_json::to_string(&counter)?), |row| {
            row.get::<_, usize>(0)
        })?;

        Ok(rows
            .next()
            .map_or(Ok(None), |v| v.map(Some))?
            .unwrap_or_default())
    }
}

impl ToSql for CounterId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

impl FromSql for CounterId {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(Self(usize::column_result(value)?))
    }
}

fn targets_for_counterspell(
    db: &Connection,
    caster: PlayerId,
    target: SpellTarget,
    targets: &mut HashSet<ActiveTarget>,
) -> anyhow::Result<()> {
    let mut cards_in_stack: Vec<(CardId, usize)> = vec![];
    let mut in_location = db.prepare(indoc! {"
            SELECT (cardid, location_seq)
            FROM cards
            WHERE location = (?1)
            ORDER BY location_seq ASC
        "})?;

    for row in in_location.query_map((serde_json::to_string(&Location::Stack)?,), |row| {
        Ok((row.get(0)?, row.get(1)?))
    })? {
        let (card, location_seq) = row?;
        cards_in_stack.push((card, location_seq))
    }

    for (card, stack_id) in cards_in_stack {
        if card.can_be_countered(db, caster, &target)? {
            targets.insert(ActiveTarget::Stack { id: stack_id });
        }
    }

    Ok(())
}

fn targets_for_battlefield_modifier(
    db: &Connection,
    source: CardId,
    modifier: Option<&BattlefieldModifier>,
    creatures: &[CardId],
    caster: PlayerId,
    targets: &mut HashSet<ActiveTarget>,
) -> anyhow::Result<()> {
    for creature in creatures.iter() {
        if creature.can_be_targeted(db, caster)?
            && (modifier.is_none()
                || creature.passes_restrictions(
                    db,
                    source,
                    caster,
                    modifier.unwrap().controller,
                    &modifier.unwrap().restrictions,
                )?)
        {
            targets.insert(ActiveTarget::Battlefield { id: *creature });
        }
    }

    Ok(())
}
