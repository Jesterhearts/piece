use serde::{Deserialize, Serialize};

use crate::{card::Effect, mana::Cost};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash, Clone)]
#[serde(deny_unknown_fields)]
pub struct Ability {
    pub cost: Cost,
    pub effects: Vec<Effect>,
}
