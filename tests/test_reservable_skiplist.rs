use mempool::mempool::mempool::{MemPool, ReservableMemPool};
use mempool::mempool::skiplist::SkipListMemPool;
use mempool::transaction::{ReservationToken, Transaction};
use std::sync::Arc;

fn tx(id: &str, fee: u64) -> Transaction {
    Transaction {
        id: id.into(),
        gas_price: fee,
        timestamp: fee,
        payload: vec![],
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn priority_order_is_respected() {
    let p = SkipListMemPool::new();
    for (id, fee) in [("a", 5), ("b", 2), ("c", 7)] {
        p.insert(tx(id, fee)).await;
    }
    let fees: Vec<_> = p.drain(3).await.into_iter().map(|t| t.gas_price).collect();
    assert_eq!(fees, vec![7, 5, 2]);
}

#[tokio::test(flavor = "multi_thread")]
async fn reserve_commit_roundtrip() {
    let p = SkipListMemPool::new();
    p.insert(tx("x", 10)).await;
    let res = p.reserve(1).await;
    let ids = res
        .txns
        .iter()
        .map(|t| Arc::from(t.id.as_str()))
        .collect::<Vec<_>>();

    // wrong token should fail
    assert!(p.commit(ReservationToken::new_v4(), &ids).await.is_empty());

    // correct token succeeds
    assert_eq!(p.commit(res.token, &ids).await.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn release_puts_tx_back() {
    let p = SkipListMemPool::new();
    p.insert(tx("y", 3)).await;
    let res = p.reserve(1).await;
    let ids = res
        .txns
        .iter()
        .map(|t| Arc::from(t.id.as_str()))
        .collect::<Vec<_>>();

    p.release(res.token, &ids).await;
    assert_eq!(p.drain(1).await.len(), 1); // Tx visible again
}

#[tokio::test(flavor = "multi_thread")]
async fn capacity_eviction_drops_lowest_fee() {
    let mut p = SkipListMemPool::new();
    p.capacity = Some(3);
    for fee in [1, 2, 3, 4] {
        p.insert(tx(&fee.to_string(), fee)).await;
    }
    assert_eq!(p.map.len(), 3);

    let lowest_remaining = p.drain(3).await.iter().map(|t| t.gas_price).min().unwrap();
    assert_eq!(lowest_remaining, 2); // fee=1 evicted
}
