use crate::mempool::mempool::MemPool;

#[derive(Clone)]
pub struct AppState<M: MemPool> {
    pub mempool: M,
}
