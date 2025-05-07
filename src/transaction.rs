use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, sync::Arc};

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub gas_price: u64,
    pub timestamp: u64,
    pub payload: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct InternalTransaction {
    pub id: Arc<str>,
    pub gas_price: u64,
    pub timestamp: u64,
    pub payload: Vec<u8>,
}

impl From<Transaction> for InternalTransaction {
    fn from(t: Transaction) -> Self {
        Self {
            id: Arc::from(t.id),
            gas_price: t.gas_price,
            timestamp: t.timestamp,
            payload: t.payload,
        }
    }
}

impl From<InternalTransaction> for Transaction {
    fn from(t: InternalTransaction) -> Self {
        Self {
            id: t.id.to_string(),
            gas_price: t.gas_price,
            timestamp: t.timestamp,
            payload: t.payload,
        }
    }
}

impl PartialOrd for InternalTransaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for InternalTransaction {
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
