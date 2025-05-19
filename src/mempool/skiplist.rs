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
use std::sync::atomic::Ordering;
use std::sync::{Arc, atomic::AtomicU8};
use uuid::Uuid;

#[derive(Clone)]
pub struct SkipListMemPool {
    pub map: Arc<SkipMap<CompositeKey, Arc<StatefulTxn>>>,
    pub reserved: DashMap<Arc<str>, (ReservationToken, Arc<StatefulTxn>)>,
    // for eviction
    pub capacity: Option<usize>,
}

impl Default for SkipListMemPool {
    fn default() -> Self {
        Self::new()
    }
}

impl SkipListMemPool {
    pub fn new() -> Self {
        Self {
            map: Arc::new(SkipMap::new()),
            reserved: DashMap::new(),
            capacity: None,
        }
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
                    if stx.state.load(Ordering::Acquire) == TxState::Available as u8 {
                        stx.state.store(TxState::Final as u8, Ordering::Release);
                    } else if stx.state.load(Ordering::Acquire) == TxState::Reserved as u8 {
                        self.map.insert(entry.key().clone(), stx.clone());
                    } else {
                        break;
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
                self.reserved
                    .insert(stx.data.id.clone(), (token, stx.clone()));
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
            if let Some((removed_key, (stored_token, stx))) = self.reserved.remove(id) {
                if token == stored_token
                    && stx
                        .state
                        .compare_exchange(
                            TxState::Reserved as u8,
                            TxState::Final as u8,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        )
                        .is_ok()
                {
                    committed.push(Transaction::from(stx.data.as_ref()));
                } else {
                    self.reserved.insert(removed_key, (stored_token, stx));
                }
            }
        }
        committed
    }
    async fn release(&self, token: ReservationToken, ids: &[Arc<str>]) {
        for id in ids {
            if let Some((_, (stored_token, stx))) = self.reserved.remove(id) {
                if stored_token == token
                    && stx
                        .state
                        .compare_exchange(
                            TxState::Reserved as u8,
                            TxState::Available as u8,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        )
                        .is_ok()
                {
                    let key = CompositeKey::from(&(*stx.data));
                    self.map.insert(key, stx);
                } else {
                    self.reserved.insert(id.clone(), (stored_token, stx));
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

        // Wait for all producers to finish
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
