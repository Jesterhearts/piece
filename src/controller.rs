use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::protogen;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone, Default, Copy)]
pub enum Controller {
    #[default]
    Any,
    You,
    Opponent,
}

impl TryFrom<&protogen::controller::Controller> for Controller {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::controller::Controller) -> Result<Self, Self::Error> {
        value
            .controller
            .as_ref()
            .ok_or_else(|| anyhow!("Expected controller to have a controller set"))
            .map(Controller::from)
    }
}

impl From<&protogen::controller::controller::Controller> for Controller {
    fn from(value: &protogen::controller::controller::Controller) -> Self {
        match value {
            protogen::controller::controller::Controller::Any(_) => Self::Any,
            protogen::controller::controller::Controller::You(_) => Self::You,
            protogen::controller::controller::Controller::Opponent(_) => Self::Opponent,
        }
    }
}
