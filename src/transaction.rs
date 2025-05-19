use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering as AtomicOrdering},
    },
};
use uuid::Uuid;

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub gas_price: u64,
    pub timestamp: u64,
    pub payload: Vec<u8>,
}

// #[repr(C)] MAYBE... Rust already optimizes aggressively
// Enforce stable memory layout like C would with repr
// This might reduce padding and reduce byte size
// println!("Size: {}", std::mem::size_of::<InternalTransaction>());
#[derive(Clone, PartialEq, Eq)]
pub struct InternalTransaction {
    pub gas_price: u64,
    pub timestamp: u64,
    pub id: Arc<str>,
    pub payload: Arc<[u8]>,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TxState {
    Available = 0,
    Reserved = 1,
    Final = 2, // committed or evicted
}

// TODO: This is brittle, implement internal methods to prevent setting it to any other value
impl From<u8> for TxState {
    fn from(v: u8) -> Self {
        // SAFETY NOTE: we never store any value other than 0,1,2
        unsafe { std::mem::transmute(v) }
    }
}

pub struct StatefulTxn {
    pub data: Arc<InternalTransaction>,
    pub state: AtomicU8,
}

impl StatefulTxn {
    fn new(tx: Transaction) -> Self {
        Self {
            data: Arc::new(InternalTransaction::from(tx)),
            state: AtomicU8::new(TxState::Available as u8),
        }
    }
}

impl From<Transaction> for InternalTransaction {
    fn from(t: Transaction) -> Self {
        Self {
            id: Arc::from(t.id),
            gas_price: t.gas_price,
            timestamp: t.timestamp,
            payload: Arc::from(t.payload),
        }
    }
}

impl From<&InternalTransaction> for Transaction {
    fn from(t: &InternalTransaction) -> Self {
        Self {
            id: t.id.to_string(),
            gas_price: t.gas_price,
            timestamp: t.timestamp,
            payload: t.payload.to_vec(),
        }
    }
}

impl From<InternalTransaction> for Transaction {
    fn from(t: InternalTransaction) -> Self {
        Self::from(&t)
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

pub type ReservationToken = Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct Reservation {
    pub token: ReservationToken,
    pub txns: Vec<Transaction>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CommitOrReleaseRequest {
    pub token: ReservationToken,
    pub txns: Vec<String>,
}
