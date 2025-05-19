use crate::{
    app_state::AppState,
    mempool::mempool::{MemPool, ReservableMemPool},
    transaction::{CommitOrReleaseRequest, Reservation, Transaction},
};
use axum::{Json, extract::State};
use std::sync::Arc;

pub async fn handle_txn_submit<M: MemPool>(
    State(state): State<AppState<M>>,
    Json(txn): Json<Transaction>,
) {
    state.mempool.insert(txn).await;
}

pub async fn handle_drain<M: MemPool>(
    State(state): State<AppState<M>>,
    Json(quantity): Json<usize>,
) -> Json<Vec<Transaction>> {
    Json(state.mempool.drain(quantity).await)
}

// Feature gated for those that implement ReservableMemPool
pub async fn handle_reserve<M: ReservableMemPool>(
    State(state): State<AppState<M>>,
    Json(quantity): Json<usize>,
) -> Json<Reservation> {
    Json(state.mempool.reserve(quantity).await)
}
pub async fn handle_commit<M: ReservableMemPool>(
    State(state): State<AppState<M>>,
    Json(CommitOrReleaseRequest { token, txns }): Json<CommitOrReleaseRequest>,
) -> Json<Vec<Transaction>> {
    let ids: Vec<Arc<str>> = txns.into_iter().map(|id| Arc::from(id)).collect();
    Json(state.mempool.commit(token, &ids).await)
}
pub async fn handle_release<M: ReservableMemPool>(
    State(state): State<AppState<M>>,
    Json(CommitOrReleaseRequest { token, txns }): Json<CommitOrReleaseRequest>,
) {
    let ids: Vec<Arc<str>> = txns.into_iter().map(|id| Arc::from(id)).collect();
    state.mempool.release(token, &ids).await;
}
