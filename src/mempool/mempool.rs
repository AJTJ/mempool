use std::sync::Arc;

use crate::transaction::{Reservation, ReservationToken, Transaction};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait MemPool: Send + Sync + 'static {
    async fn insert(&self, tx: Transaction);
    async fn drain(&self, n: usize) -> Vec<Transaction>;
}

#[async_trait]
pub trait ReservableMemPool: MemPool {
    async fn reserve(&self, n: usize) -> Reservation;
    async fn commit(&self, token: ReservationToken, ids: &[Arc<str>]) -> Vec<Transaction>;
    async fn release(&self, token: ReservationToken, ids: &[Arc<str>]);
}
