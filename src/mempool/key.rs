use std::{
    cmp::{Ordering, Reverse},
    sync::Arc,
};

#[derive(PartialEq, Eq, Clone)]
pub struct CompositeKey {
    pub gas_price: u64,
    pub timestamp: u64,
    pub id: Arc<str>,
}

impl PartialOrd for CompositeKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CompositeKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.gas_price
            .cmp(&other.gas_price) // higher
            .then_with(|| {
                other
                    .timestamp // earlier
                    .cmp(&self.timestamp)
            })
            .then_with(|| self.id.cmp(&other.id))
    }
}
