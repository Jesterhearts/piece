use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, CastFrom, Database},
    log::Log,
    protogen::{
        effects::{
            ChooseCast, CompleteSpellResolution, MoveToBattlefield, MoveToExile, MoveToGraveyard,
            PopSelected, SelectSource, TriggeredAbility,
        },
        targets::Location,
    },
    turns::Phase,
};

impl EffectBehaviors for CompleteSpellResolution {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        _selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let card = source.unwrap();
        Log::spell_resolved(db, card);

        if card.is_in_location(db, Location::IN_STACK) {
            let effects = if card.is_permanent(db) {
                vec![
                    MoveToBattlefield::default().into(),
                    PopSelected::default().into(),
                    PopSelected::default().into(),
                ]
            } else if card.rebound(db) && db[card].cast_from == Some(CastFrom::Hand) {
                db.delayed_triggers
                    .entry(db[card].owner)
                    .or_default()
                    .entry(Phase::Upkeep)
                    .or_default()
                    .push((
                        card,
                        TriggeredAbility {
                            effects: vec![
                                SelectSource::default().into(),
                                ChooseCast::default().into(),
                            ],
                            oracle_text: "At the beginning of your next upkeep, \
                                you may cast the spell from exile without paying its mana cost"
                                .to_string(),
                            ..Default::default()
                        },
                    ));

                vec![MoveToExile::default().into(), PopSelected::default().into()]
            } else {
                vec![
                    MoveToGraveyard::default().into(),
                    PopSelected::default().into(),
                ]
            };

            vec![EffectBundle {
                effects,
                source,
                ..Default::default()
            }]
        } else {
            vec![]
        }
    }
}
