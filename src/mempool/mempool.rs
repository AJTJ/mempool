use crate::transaction::Transaction;
use async_trait::async_trait;

#[async_trait]
pub trait MemPool: Send + Sync + 'static {
    async fn insert(&self, tx: Transaction);
    async fn drain(&self, n: usize) -> Vec<Transaction>;
}
