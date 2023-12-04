use std::sync::atomic::AtomicUsize;

use bevy_ecs::component::Component;
use derive_more::From;

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerId(usize);

impl PlayerId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

impl PartialEq<Controller> for PlayerId {
    fn eq(&self, other: &Controller) -> bool {
        *self == other.0
    }
}

impl PartialEq<Owner> for PlayerId {
    fn eq(&self, other: &Owner) -> bool {
        *self == other.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component)]
pub struct Controller(pub PlayerId);

impl From<Owner> for Controller {
    fn from(value: Owner) -> Self {
        Self(value.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Component, From)]
pub struct Owner(pub PlayerId);
