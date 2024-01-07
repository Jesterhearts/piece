use std::collections::HashMap;

use indexmap::IndexSet;

use crate::{
    in_play::CardId,
    player::{Controller, Owner},
};

#[derive(Debug, Default)]
pub struct Hand {
    pub(crate) hands: HashMap<Owner, IndexSet<CardId>>,
}

impl std::ops::Index<Owner> for Hand {
    type Output = IndexSet<CardId>;

    fn index(&self, index: Owner) -> &Self::Output {
        self.hands.get(&index).unwrap()
    }
}

impl std::ops::Index<Controller> for Hand {
    type Output = IndexSet<CardId>;

    fn index(&self, index: Controller) -> &Self::Output {
        self.hands.get(&Owner::from(index)).unwrap()
    }
}

impl std::ops::IndexMut<Owner> for Hand {
    fn index_mut(&mut self, index: Owner) -> &mut Self::Output {
        self.hands.entry(index).or_default()
    }
}

impl std::ops::IndexMut<Controller> for Hand {
    fn index_mut(&mut self, index: Controller) -> &mut Self::Output {
        self.hands.entry(Owner::from(index)).or_default()
    }
}
