use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use mempool::{
    mempool::{
        binary_heap::BHeapMemPool, btree::BTreeMemPool, mempool::MemPool, skiplist::SkipListMemPool,
    },
    transaction::Transaction,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use uuid::Uuid;

async fn push_txns<P: MemPool + ?Sized>(pool: &P, i: usize, n: usize) {
    for j in 0..n {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        pool.insert(Transaction {
            id: Uuid::new_v4().to_string(),
            gas_price: (i + j) as u64,
            timestamp: now,
            payload: vec![1, 2],
        })
        .await;
    }
}

fn bench_insert(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("insert transactions");
    let sizes = [1_000, 5_000];

    for size in sizes {
        // Benchmark BTreeMemPool
        group.bench_function(BenchmarkId::new("btree", size), |b| {
            b.iter(|| {
                rt.block_on(async {
                    let pool = BTreeMemPool::default();
                    push_txns(&pool, 0, size).await;
                })
            })
        });

        // Benchmark BHeapMemPool
        group.bench_function(BenchmarkId::new("binary_heap", size), |b| {
            b.iter(|| {
                rt.block_on(async {
                    let pool = BHeapMemPool::default();
                    push_txns(&pool, 0, size).await;
                })
            })
        });

        // Benchmark SkipListMemPool
        group.bench_function(BenchmarkId::new("skiplist", size), |b| {
            b.iter(|| {
                rt.block_on(async {
                    let pool = SkipListMemPool::default();
                    push_txns(&pool, 0, size).await;
                })
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_insert);
criterion_main!(benches);
