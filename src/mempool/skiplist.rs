use super::mempool::MemPool;
use crate::transaction::{InternalTransaction, Transaction};
use async_trait::async_trait;
use crossbeam_skiplist::{SkipMap, SkipSet};
use std::sync::Arc;

#[derive(Clone)]
pub struct SkipListMemPool {
    set: Arc<SkipSet<InternalTransaction>>,
}

impl Default for SkipListMemPool {
    fn default() -> Self {
        Self::new()
    }
}

impl SkipListMemPool {
    pub fn new() -> Self {
        Self {
            set: Arc::new(SkipSet::new()),
        }
    }
}

#[async_trait]
impl MemPool for SkipListMemPool {
    async fn insert(&self, t: Transaction) {
        let i = InternalTransaction::from(t);
        self.set.insert(i);
    }

    async fn drain(&self, n: usize) -> Vec<Transaction> {
        if n == 0 {
            return Vec::new();
        }

        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            if let Some(entry) = self.set.pop_back() {
                out.push(entry.key().clone());
            } else {
                break;
            }
        }

        out.into_iter().map(Transaction::from).collect()
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

                    pool.clone().insert(txn).await;
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
