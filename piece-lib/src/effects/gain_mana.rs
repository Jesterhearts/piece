use crate::{
    effects::{EffectBehaviors, EffectBundle, Options, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    protogen::effects::{gain_mana::Gain, GainMana},
    stack::Selected,
};

impl EffectBehaviors for GainMana {
    fn wants_input(
        &self,
        _db: &Database,
        _source: Option<CardId>,
        _already_selected: &[Selected],
        _modes: &[usize],
    ) -> bool {
        match self.gain.as_ref().unwrap() {
            Gain::Specific(_) => false,
            Gain::Choice(_) => true,
        }
    }

    fn options(
        &self,
        _db: &Database,
        _source: Option<CardId>,
        _already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
        match self.gain.as_ref().unwrap() {
            Gain::Specific(_) => Options::ListWithDefault(vec![]),
            Gain::Choice(choice) => {
                let mut options = vec![];
                for (idx, choice) in choice.choices.iter().enumerate() {
                    let mut add = "Add ".to_string();
                    for mana in choice.gains.iter() {
                        mana.enum_value().unwrap().push_mana_symbol(&mut add);
                    }
                    options.push((idx, add));
                }

                Options::MandatoryList(options)
            }
        }
    }

    fn select(
        &mut self,
        _db: &mut Database,
        _source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
    ) -> SelectionResult {
        if let Some(option) = option {
            selected.modes.push(option);
            SelectionResult::Complete
        } else {
            SelectionResult::PendingChoice
        }
    }

    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        match self.gain.as_ref().unwrap() {
            Gain::Specific(gain) => {
                let controller = db[source.unwrap()].controller;
                for gain in gain.gain.iter() {
                    db.all_players[controller].mana_pool.apply(
                        gain.enum_value().unwrap(),
                        self.mana_source.enum_value().unwrap(),
                        self.mana_restriction.enum_value().unwrap(),
                    );
                }
            }
            Gain::Choice(choice) => {
                let mode = selected.modes.first().unwrap();
                let chosen = &choice.choices[*mode];
                let controller = db[source.unwrap()].controller;
                for gain in chosen.gains.iter() {
                    db.all_players[controller].mana_pool.apply(
                        gain.enum_value().unwrap(),
                        self.mana_source.enum_value().unwrap(),
                        self.mana_restriction.enum_value().unwrap(),
                    );
                }
            }
        }

        vec![]
    }
}
