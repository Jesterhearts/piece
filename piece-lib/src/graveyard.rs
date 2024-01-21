use std::collections::HashMap;

use indexmap::{IndexMap, IndexSet};

use crate::protogen::ids::{CardId, Controller, Owner};

#[derive(Debug, Default)]
pub struct Graveyards {
    pub(crate) graveyards: IndexMap<Owner, IndexSet<CardId>>,
    pub(crate) descended_this_turn: HashMap<Owner, usize>,
}

impl std::ops::Index<&Owner> for Graveyards {
    type Output = IndexSet<CardId>;

    fn index(&self, index: &Owner) -> &Self::Output {
        self.graveyards.get(index).unwrap()
    }
}

impl std::ops::Index<&Controller> for Graveyards {
    type Output = IndexSet<CardId>;

    fn index(&self, index: &Controller) -> &Self::Output {
        self.graveyards.get(&Owner::from(index.clone())).unwrap()
    }
}

impl std::ops::IndexMut<&Owner> for Graveyards {
    fn index_mut(&mut self, index: &Owner) -> &mut Self::Output {
        self.graveyards.entry(index.clone()).or_default()
    }
}

impl std::ops::IndexMut<&Controller> for Graveyards {
    fn index_mut(&mut self, index: &Controller) -> &mut Self::Output {
        self.graveyards
            .entry(Owner::from(index.clone()))
            .or_default()
    }
}
