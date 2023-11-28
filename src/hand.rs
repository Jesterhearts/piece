use std::rc::Rc;

use crate::card::Card;

#[derive(Debug)]
pub struct Hand {
    pub max_size: usize,
    pub contents: Vec<Rc<Card>>,
}

impl Default for Hand {
    fn default() -> Self {
        Self {
            max_size: 7,
            contents: Default::default(),
        }
    }
}
