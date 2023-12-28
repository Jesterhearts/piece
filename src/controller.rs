use anyhow::anyhow;
use bevy_ecs::component::Component;

use crate::protogen;

#[derive(Debug, PartialEq, Eq, Clone, Default, Copy, Component)]
pub(crate) enum ControllerRestriction {
    #[default]
    Any,
    You,
    Opponent,
}

impl TryFrom<&protogen::controller::Controller> for ControllerRestriction {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::controller::Controller) -> Result<Self, Self::Error> {
        value
            .controller
            .as_ref()
            .ok_or_else(|| anyhow!("Expected controller to have a controller set"))
            .map(ControllerRestriction::from)
    }
}

impl From<&protogen::controller::controller::Controller> for ControllerRestriction {
    fn from(value: &protogen::controller::controller::Controller) -> Self {
        match value {
            protogen::controller::controller::Controller::Any(_) => Self::Any,
            protogen::controller::controller::Controller::You(_) => Self::You,
            protogen::controller::controller::Controller::Opponent(_) => Self::Opponent,
        }
    }
}
