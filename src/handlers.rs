use crate::{app_state::AppState, mempool::mempool::MemPool, transaction::Transaction};
use axum::{Json, extract::State};

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
