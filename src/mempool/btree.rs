use crate::transaction::{InternalTransaction, Transaction};
use async_trait::async_trait;
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;

use super::{key::CompositeKey, mempool::MemPool};

// TODO consider parking_lot mutex

#[derive(Default, Clone)]
pub struct BTreeMemPool {
    data: Arc<Mutex<BTreeMap<CompositeKey, InternalTransaction>>>,
}

#[async_trait]
impl MemPool for BTreeMemPool {
    async fn insert(&self, t: Transaction) {
        let internal_tx = InternalTransaction::from(t);
        let mut data = self.data.lock().await;
        data.insert(
            CompositeKey {
                gas_price: internal_tx.gas_price,
                timestamp: internal_tx.timestamp,
                id: internal_tx.id.clone(),
            },
            internal_tx,
        );
    }

    async fn drain(&self, n: usize) -> Vec<Transaction> {
        let drained = self.perform_drain(n).await;
        drained.into_iter().map(Transaction::from).collect()
    }
}

impl BTreeMemPool {
    async fn perform_drain(&self, n: usize) -> Vec<InternalTransaction> {
        let mut data = self.data.lock().await;
        if n == 0 || data.is_empty() {
            return Vec::new();
        }

        let mut drained = Vec::with_capacity(n);
        let keys: Vec<_> = data.keys().rev().take(n).cloned().collect();

        for key in keys {
            if let Some(tx) = data.remove(&key) {
                drained.push(tx);
            }
        }

        drained
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_b_tree() {
        let pool = BTreeMemPool::default();
        let mut handles = Vec::new();
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

                    pool.clone().insert(txn).await;
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // test basic drain
        let drained = pool.clone().drain(2).await;
        assert_eq!(drained.len(), 2);

        // Drain over the limit, i.e. the remaining
        let over_drain = pool.drain(100).await;
        assert_eq!(over_drain.len(), 23)
    }
}
