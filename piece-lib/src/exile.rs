use indexmap::{IndexMap, IndexSet};

use crate::{
    player::{Controller, Owner},
    protogen::ids::CardId,
};

#[derive(Debug, Default)]
pub struct Exiles {
    pub(crate) exile_zones: IndexMap<Owner, IndexSet<CardId>>,
}

impl std::ops::Index<Owner> for Exiles {
    type Output = IndexSet<CardId>;

    fn index(&self, index: Owner) -> &Self::Output {
        self.exile_zones.get(&index).unwrap()
    }
}

impl std::ops::Index<Controller> for Exiles {
    type Output = IndexSet<CardId>;

    fn index(&self, index: Controller) -> &Self::Output {
        self.exile_zones.get(&Owner::from(index)).unwrap()
    }
}

impl std::ops::IndexMut<Owner> for Exiles {
    fn index_mut(&mut self, index: Owner) -> &mut Self::Output {
        self.exile_zones.entry(index).or_default()
    }
}

impl std::ops::IndexMut<Controller> for Exiles {
    fn index_mut(&mut self, index: Controller) -> &mut Self::Output {
        self.exile_zones.entry(Owner::from(index)).or_default()
    }
}
