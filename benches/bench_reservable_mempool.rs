use criterion::{Criterion, criterion_group, criterion_main};
use mempool::mempool::mempool::ReservableMemPool;

use mempool::mempool::{mempool::MemPool, skiplist::SkipListMemPool};
use mempool::transaction::Transaction;
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{runtime::Runtime, time::sleep};
use uuid::Uuid;

fn now_sec() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn make_tx(idx: u64) -> Transaction {
    Transaction {
        id: Uuid::new_v4().to_string(),
        gas_price: idx,
        timestamp: now_sec(),
        payload: vec![1, 2],
    }
}

async fn skiplist_heavy(pool: Arc<SkipListMemPool>) {
    // 4 producers
    let mut joins = Vec::new();
    for p in 0..4 {
        let clone = pool.clone();
        joins.push(tokio::spawn(async move {
            for i in 0..5_000u64 {
                clone.insert(make_tx(i + (p as u64) * 5_000)).await;
            }
        }));
    }

    // builder
    let builder_pool = pool.clone();
    let builder = tokio::spawn(async move {
        for _ in 0..100 {
            let res = builder_pool.reserve(100).await;
            let ids = res
                .txns
                .iter()
                .map(|t| Arc::from(t.id.as_str()))
                .collect::<Vec<_>>();
            sleep(Duration::from_micros(50)).await;
            builder_pool.commit(res.token, &ids).await;
        }
    });

    for j in joins {
        j.await.unwrap();
    }
    builder.await.unwrap();
}

fn bench_skiplist_two_step(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = Arc::new(SkipListMemPool::default());

    c.bench_function("skiplist_two_step", |b| {
        b.to_async(&rt).iter(|| skiplist_heavy(pool.clone()));
    });
}

criterion_group!(benches, bench_skiplist_two_step);
criterion_main!(benches);
