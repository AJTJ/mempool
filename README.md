## What is this?
- This is an implementation of multiple mempool data structure strategies commonly used in high-throughput systems.

## Usage
- `cargo bench`
  - this runs all of the Criterion benches located in /benches (should take a minute or so). These benches use and compare all the mempool implementations.
  - To get a visualization of the benches afterwards open: `target/criterion/report/index.html`
- `cargo run`
  - to run the server and access the endpoints
- `cargo run --no-default-features --features mempool-btree`
  - to run the server using the btree (or another feature)
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

## Notes on the Benches
- The `SkipSet` generally generally seems to perform a bit better when drains are involved, likely to do it isn't single threaded and avoids the request/reply round trip.
  - I did experiment with `SkipMap` too, and it also seems to perform better than the `SkipSet` sometimes. I'd need further research, and perhaps better benches.
- The BinaryHeap generally excels on pure insertions, as producers never grab a lock, they just enqueue so inserts scale with CPU core.

## Some Optimizations
- `Arc<str>` inside `InternalTransaction` so that cloning an ID is one Atomic increment
- Pre-allocation of return vectors with `with_capacity(n)` avoid reallocating during large drains.
- Will likely dig into further code optimizations where useful

## Mempools used
### `BinaryHeap`
- O(log n) inserts, O(k log N) drains
- Lock-free due to the mpsc channel implementation and being bound by a single core
- Wins with insertions, in terms of raw performance until the single heap core saturates

### `Crossbeam SkipSet`
- O(log n) average time complexity for both inserts and drains
- lock-free concurrent access through atomic operations
- Scalable, multi-threaded and no need for manual thread management or channel communication

### `BinaryTreeMap` (not as good, but interesting)
- O(log n) inserts + mutex lock, O(k log N) drains, but also lock
- Interesting, but certainly limited by having a locking data struture


## Some Tradeoffs
- Initially I was curious about the performance of the BinaryTreeMap since it is not widely used (for obvious reasons), but the locking data structure is too much
- A BinaryHeap is excellent until the single process can't handle the throughput anymore, there is a distinct tradeoff in that regard
- The SkipSet performs the best, except on pure insertions, where the binary heap shines. That being said, the SkipSet also natively offers more features, as needed, such as delete/replace by ID



## Next Steps
- Likely the most performant current alternative (with the current spec) would be sharded binary heaps with a k-way merge on drain.
- 