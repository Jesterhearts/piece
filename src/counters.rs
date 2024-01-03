use std::str::FromStr;

use anyhow::{anyhow, Context};

use crate::newtype_enum::newtype_enum;

newtype_enum! {
#[derive(Debug, Clone, Copy, PartialEq, Eq, bevy_ecs::component::Component)]
#[derive(strum::EnumIter, strum::AsRefStr, strum::EnumString, Hash)]
pub enum Counter {
    Any,
    Charge,
    Net,
    P1P1,
    M1M1,
}
}

impl TryFrom<&String> for Counter {
    type Error = anyhow::Error;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        match value.trim() {
            "+1/+1" => Ok(Self::P1P1),
            "-1/-1" => Ok(Self::M1M1),
            other => {
                Ok(Self::from_str(other).with_context(|| anyhow!("Parsing counter {}", value))?)
            }
        }
    }
}
