use criterion::{Criterion, criterion_group, criterion_main};
use mempool::{
    mempool::{
        binary_heap::BHeapMemPool, btree::BTreeMemPool, mempool::MemPool, skiplist::SkipListMemPool,
    },
    transaction::Transaction,
};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use uuid::Uuid;

async fn run_benchmark_scenario(pool: Arc<dyn MemPool>) {
    let mut handles = Vec::new();

    for i in 0..5 {
        let pool = pool.clone();
        let handle = tokio::spawn(async move {
            for j in 0..900 {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let txn = Transaction {
                    id: Uuid::new_v4().to_string(),
                    gas_price: i + j,
                    timestamp: now,
                    payload: vec![1, 2],
                };

                pool.insert(txn).await;
            }

            pool.drain(50).await;

            for j in 900..1000 {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let txn = Transaction {
                    id: Uuid::new_v4().to_string(),
                    gas_price: i + j,
                    timestamp: now,
                    payload: vec![1, 2],
                };

                pool.insert(txn).await;
            }

            pool.drain(50).await;
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

fn bench_insert_and_drain(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("insert and drain transactions");

    // Benchmark BTreeMemPool
    group.bench_function("btree", |b| {
        b.iter(|| {
            rt.block_on(async {
                let pool: Arc<dyn MemPool> = Arc::new(BTreeMemPool::default());
                run_benchmark_scenario(pool).await;
            })
        });
    });

    // Benchmark BHeapMemPool
    group.bench_function("binary_heap", |b| {
        b.iter(|| {
            rt.block_on(async {
                let pool: Arc<dyn MemPool> = Arc::new(BHeapMemPool::default());
                run_benchmark_scenario(pool).await;
            })
        });
    });

    // Benchmark SkipListMemPool
    group.bench_function("skiplist", |b| {
        b.iter(|| {
            rt.block_on(async {
                let pool: Arc<dyn MemPool> = Arc::new(SkipListMemPool::default());
                run_benchmark_scenario(pool).await;
            })
        });
    });

    group.finish();
}

criterion_group!(benches, bench_insert_and_drain);
criterion_main!(benches);
