## What is this?
- This is an implementation of multiple mempool data structure strategies commonly used in high-throughput systems.

## Usage
- `cargo bench`
  - this runs all of the Criterion benches located in /benches (should take a minute or so)
  - To get a visualization of the benches afterwards open: `target/criterion/report/index.html`
- `cargo run`
  - to run the server and access the endpoints
- `cargo run --no-default-features --features mempool-btree`
  - to run the server using the btree (or another future feature)
  - NOTE: feature flags located in Cargo.toml
- `cargo test`
  - to test the default feature
  - And also ensure correct ordering
- `cargo test --no-default-features --features mempool-btree`
  - to test the btree implementation (or another feature)


## Overview
- The entire service is wrapped in a multi-threaded Axum server exposing two endpoints via a REST API
- `submit(tx: Transaction)`
  - simulates submitting a transaction into the mempool.
- `drain(n: usize) -> Vec<Transaction>`
  - returns the top-N prioritized transactions and removes them from the mempool.
- The mempool implementions prioritize transactions based on gas price (higher = higher priority) and timestamp (earlier = higher priority if gas prices are equal).
- Prioritization logic is handled in the `Ord` implementations in both the `InternalTransaction` and `CompositeKey` structs

## Some Optimizations (BinaryHeap mostly)
- Producers never grab a lock, they just enqueue so inserts scale with CPU core
- `Arc<str>` inside `InternalTransaction` so that cloning an ID is one Atomic increment
- Pre-allocation of return vectors with `with_capacity(n)` avoid reallocating during large drains

## Mempools used
### `BinaryHeap`
- O(log n) inserts, O(k log N) drains
- Lock-free due to the mpsc channel implementation and being bound by a single core
- Wins in terms of raw performance until the single heap core saturates
- This data structure would also be easily adaptable to a remove(ID) service

### `BinaryTreeMap` (not as good, but interesting)
- O(log n) inserts + mutex lock, O(k log N) drains, but also lock
- Interesting, but certainly limited by having a locking data struture

## Tradeoffs And future thoughts
- Initially I was curious about the performance of the BinaryTreeMap since it is not widely used (for obvious reasons), but the locking data structure is too much
- A BinaryHeap is excellent until the single process can't handle the throughput anymore, there is a distinct tradeoff in that regard

## Other likely more performant Mempool implementations to explore
- Concurrent priority queue via crossbeam, mpmcpq etc...
- Atomic skip-list (likely the best)
- Sharded buckets via dashmap