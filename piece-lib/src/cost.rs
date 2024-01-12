use crate::protogen::{
    color::Color,
    cost::{CastingCost, ManaCost},
};

impl CastingCost {
    pub(crate) fn colors(&self) -> Vec<Color> {
        self.mana_cost
            .iter()
            .map(|mana| mana.enum_value().unwrap().color())
            .collect()
    }

    pub fn text(&self) -> String {
        let mut result = String::default();

        let generic = self
            .mana_cost
            .iter()
            .filter(|cost| matches!(cost.enum_value().unwrap(), ManaCost::GENERIC))
            .count();

        let mut pushed_generic = false;
        for mana in self.mana_cost.iter() {
            match mana.enum_value().unwrap() {
                ManaCost::WHITE => result.push('\u{e600}'),
                ManaCost::BLUE => result.push('\u{e601}'),
                ManaCost::BLACK => result.push('\u{e602}'),
                ManaCost::RED => result.push('\u{e603}'),
                ManaCost::GREEN => result.push('\u{e604}'),
                ManaCost::COLORLESS => result.push('\u{e904}'),
                ManaCost::GENERIC => {
                    if !pushed_generic {
                        match generic {
                            0 => result.push('\u{e605}'),
                            1 => result.push('\u{e606}'),
                            2 => result.push('\u{e607}'),
                            3 => result.push('\u{e608}'),
                            4 => result.push('\u{e609}'),
                            5 => result.push('\u{e60a}'),
                            6 => result.push('\u{e60b}'),
                            7 => result.push('\u{e60c}'),
                            8 => result.push('\u{e60d}'),
                            9 => result.push('\u{e60e}'),
                            10 => result.push('\u{e60f}'),
                            11 => result.push('\u{e610}'),
                            12 => result.push('\u{e611}'),
                            13 => result.push('\u{e612}'),
                            14 => result.push('\u{e613}'),
                            15 => result.push('\u{e614}'),
                            16 => result.push('\u{e62a}'),
                            17 => result.push('\u{e62b}'),
                            18 => result.push('\u{e62c}'),
                            19 => result.push('\u{e62d}'),
                            20 => result.push('\u{e62e}'),
                            _ => result.push_str(&format!("{}", generic)),
                        }
                        pushed_generic = true;
                    }
                }
                ManaCost::X => result.push('\u{e615}'),
                ManaCost::TWO_X => result.push_str("\u{e615}\u{e615}"),
            }
        }

        result
    }

    pub(crate) fn cmc(&self) -> usize {
        self.mana_cost.len()
    }
}
