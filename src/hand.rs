use crate::in_play::CardId;

#[derive(Debug)]
pub struct Hand {
    pub max_size: usize,
    pub contents: Vec<CardId>,
}

impl Default for Hand {
    fn default() -> Self {
        Self {
            max_size: 7,
            contents: Default::default(),
        }
    }
}
