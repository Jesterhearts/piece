use indexmap::{IndexMap, IndexSet};

use crate::{
    in_play::CardId,
    player::{Controller, Owner},
};

#[derive(Debug, Default)]
pub struct Hands {
    pub(crate) hands: IndexMap<Owner, IndexSet<CardId>>,
}

impl std::ops::Index<Owner> for Hands {
    type Output = IndexSet<CardId>;

    fn index(&self, index: Owner) -> &Self::Output {
        self.hands.get(&index).unwrap()
    }
}

impl std::ops::Index<Controller> for Hands {
    type Output = IndexSet<CardId>;

    fn index(&self, index: Controller) -> &Self::Output {
        self.hands.get(&Owner::from(index)).unwrap()
    }
}

impl std::ops::IndexMut<Owner> for Hands {
    fn index_mut(&mut self, index: Owner) -> &mut Self::Output {
        self.hands.entry(index).or_default()
    }
}

impl std::ops::IndexMut<Controller> for Hands {
    fn index_mut(&mut self, index: Controller) -> &mut Self::Output {
        self.hands.entry(Owner::from(index)).or_default()
    }
}
