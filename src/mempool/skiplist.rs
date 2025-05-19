use super::{
    key::CompositeKey,
    mempool::{MemPool, ReservableMemPool},
};
use crate::transaction::{
    InternalTransaction, Reservation, ReservationToken, StatefulTxn, Transaction, TxState,
};
use async_trait::async_trait;
use crossbeam_skiplist::SkipMap;
use dashmap::DashMap;
use std::{sync::atomic::Ordering, time::Instant};
use std::{
    sync::{Arc, atomic::AtomicU8},
    time::Duration,
};
use tokio::time::sleep;
use uuid::Uuid;

// TODO: Make this configurable per reservation
const DEFAULT_TTL: u64 = 2000;

#[derive(Clone)]
pub struct ReservedEntry {
    pub token: ReservationToken,
    pub stx: Arc<StatefulTxn>,
    pub expires: Instant,
}

#[derive(Clone)]
pub struct SkipListMemPool {
    pub map: Arc<SkipMap<CompositeKey, Arc<StatefulTxn>>>,
    pub reserved: DashMap<Arc<str>, ReservedEntry>,
    pub capacity: Option<usize>,
}

impl Default for SkipListMemPool {
    fn default() -> Self {
        Self::new()
    }
}

impl SkipListMemPool {
    pub fn new() -> Self {
        let new = Self {
            map: Arc::new(SkipMap::new()),
            reserved: DashMap::new(),
            capacity: None,
        };

        let map_ref = new.map.clone();
        let reserved_ref = new.reserved.clone();

        // reaper task
        tokio::spawn(async move {
            let sweep_delay = Duration::from_millis(DEFAULT_TTL / 4);
            loop {
                // sleep first
                sleep(sweep_delay).await;
                let now = Instant::now();
                reserved_ref.retain(|_, entry| {
                    if entry.expires <= now {
                        if entry
                            .stx
                            .state
                            .compare_exchange(
                                TxState::Reserved as u8,
                                TxState::Available as u8,
                                Ordering::AcqRel,
                                Ordering::Acquire,
                            )
                            .is_ok()
                        {
                            let key = CompositeKey::from(&*entry.stx.data);
                            map_ref.insert(key, entry.stx.clone());
                        }
                        // drops
                        false
                    } else {
                        // keeps
                        true
                    }
                });
            }
        });
        new
    }
}

impl SkipListMemPool {
    fn get_n_txns(&self, n: usize) -> Vec<Arc<StatefulTxn>> {
        if n == 0 {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            match self.map.pop_back() {
                Some(entry) => out.push(entry.value().clone()),
                None => break,
            }
        }
        out
    }
}

#[async_trait]
impl MemPool for SkipListMemPool {
    async fn insert(&self, t: Transaction) {
        let internal = InternalTransaction::from(t);
        let key = CompositeKey::from(&internal);
        let stx = Arc::new(StatefulTxn {
            data: Arc::new(internal),
            state: AtomicU8::new(TxState::Available as u8),
        });
        self.map.insert(key, stx);

        if let Some(max) = self.capacity {
            while self.map.len() > max {
                if let Some(entry) = self.map.pop_front() {
                    let stx = entry.value();
                    let cur = stx.state.load(Ordering::Acquire);

                    match cur {
                        v if v == TxState::Available as u8 => {
                            stx.state.store(TxState::Final as u8, Ordering::Release)
                        }
                        v if v == TxState::Reserved as u8 => {
                            self.map.insert(entry.key().clone(), stx.clone());
                        }
                        _ => break,
                    }
                }
            }
        }
    }

    async fn drain(&self, n: usize) -> Vec<Transaction> {
        let res = self.reserve(n).await;
        let ids: Vec<Arc<str>> = res.txns.iter().map(|t| Arc::from(t.id.as_str())).collect();
        self.commit(res.token, &ids).await
    }
}

#[async_trait]
impl ReservableMemPool for SkipListMemPool {
    async fn reserve(&self, n: usize) -> Reservation {
        let token = Uuid::new_v4();
        let mut reservation_tx = Vec::with_capacity(n);
        for stx in self.get_n_txns(n) {
            if stx
                .state
                .compare_exchange(
                    TxState::Available as u8,
                    TxState::Reserved as u8,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                let entry = ReservedEntry {
                    token,
                    stx: stx.clone(),
                    expires: Instant::now() + Duration::from_millis(DEFAULT_TTL),
                };
                self.reserved.insert(stx.data.id.clone(), entry);
                reservation_tx.push(Transaction::from(stx.data.as_ref()));
            }
        }

        Reservation {
            token,
            txns: reservation_tx,
        }
    }

    async fn commit(&self, token: ReservationToken, ids: &[Arc<str>]) -> Vec<Transaction> {
        let mut committed = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some((removed_key, entry)) = self.reserved.remove(id) {
                if token == entry.token
                    && entry
                        .stx
                        .state
                        .compare_exchange(
                            TxState::Reserved as u8,
                            TxState::Final as u8,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        )
                        .is_ok()
                {
                    committed.push(Transaction::from(entry.stx.data.as_ref()));
                } else {
                    self.reserved.insert(removed_key, entry);
                }
            }
        }
        committed
    }

    async fn release(&self, token: ReservationToken, ids: &[Arc<str>]) {
        for id in ids {
            if let Some((_, entry)) = self.reserved.remove(id) {
                if entry.token == token
                    && entry
                        .stx
                        .state
                        .compare_exchange(
                            TxState::Reserved as u8,
                            TxState::Available as u8,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        )
                        .is_ok()
                {
                    let key = CompositeKey::from(&*entry.stx.data);
                    self.map.insert(key, entry.stx);
                } else {
                    self.reserved.insert(id.clone(), entry);
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_skiplist() {
        let pool = SkipListMemPool::new();
        let mut handles = Vec::new();

        // Spawn multiple producers
        for i in 0..5 {
            let pool = pool.clone();
            let handle = tokio::spawn(async move {
                for j in 0..5 {
                    let txn = Transaction {
                        id: format!("{i}: {j}"),
                        gas_price: i + j,
                        timestamp: i + j,
                        payload: vec![1, 2],
                    };

                    pool.insert(txn).await;
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // Test basic drain
        let drained = pool.clone().drain(2).await;
        assert_eq!(drained.len(), 2);

        // Drain over the limit, i.e. the remaining
        let over_drain = pool.drain(100).await;
        assert_eq!(over_drain.len(), 23);
    }
}
