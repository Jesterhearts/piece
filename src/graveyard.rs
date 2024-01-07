use std::collections::HashMap;

use indexmap::IndexSet;

use crate::{
    in_play::CardId,
    player::{Controller, Owner},
};

#[derive(Debug, Default)]
pub struct Graveyard {
    pub(crate) graveyards: HashMap<Owner, IndexSet<CardId>>,
    pub(crate) descended_this_turn: HashMap<Owner, usize>,
}

impl std::ops::Index<Owner> for Graveyard {
    type Output = IndexSet<CardId>;

    fn index(&self, index: Owner) -> &Self::Output {
        self.graveyards.get(&index).unwrap()
    }
}

impl std::ops::Index<Controller> for Graveyard {
    type Output = IndexSet<CardId>;

    fn index(&self, index: Controller) -> &Self::Output {
        self.graveyards.get(&Owner::from(index)).unwrap()
    }
}

impl std::ops::IndexMut<Owner> for Graveyard {
    fn index_mut(&mut self, index: Owner) -> &mut Self::Output {
        self.graveyards.entry(index).or_default()
    }
}

impl std::ops::IndexMut<Controller> for Graveyard {
    fn index_mut(&mut self, index: Controller) -> &mut Self::Output {
        self.graveyards.entry(Owner::from(index)).or_default()
    }
}
