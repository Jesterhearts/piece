use bevy_ecs::component::Component;

use crate::protogen;

#[derive(Debug, PartialEq, Eq, Clone, Default, Copy, Component)]
pub enum Controller {
    #[default]
    Any,
    You,
    Opponent,
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
