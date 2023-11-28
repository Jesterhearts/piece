use serde::{Deserialize, Serialize};

use crate::{card::Effect, mana::Cost};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Hash, Clone)]
pub struct Ability {
    pub cost: Cost,
    pub effect: Effect,
}
