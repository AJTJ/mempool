
## V2

## Overview
- I was curious about adding a 2-step drain process while maintaining a lock-free environment
- And also implementing a basic eviction policy
- I implemented this over the `SkipListMemPool`, since based on my previous experiments that it was the most likely candidate for this process.
- These new RESTful APIs are exposed when the default feature flag is enabled.
- I also implemented a couple more efficiency improvements.
- Added a few benches to test the "reservable" features.

## Notes on Usage
- Usage stays the same as below (in `V1`), but it is only the default feature (the `SkipListMemPool`) that uses the `ReservableMemPool` behavior and endpoints.

## NEW: ReservableMemPool trait
- Added a `ReservableMemPool` trait that is implemented for the `SkipListMemPool`
- This includes reserve/commit/release methods for a two-step drain process

## Changes to the `SkipListMemPool`
- Switched it from a `SkipSet` to a `SkipMap` in or to separate ordering logic from stateful logic.
- It has a separate `reserved` data structure: `DashMap` to store to separate reserved txns from the ordered list of txns.
- It uses Atomic operations to maintain lock-free transitioning between the `map` and the `reserved` list, ensuring strong, safe concurrency.
- It also includes an optional eviction policy to manage the memory imprint.
- A reaper task runs in the background to ensure expired txns are removed from the `reserved` map and back into the main `map`.

## Other improvements
- The `InternalTransaction` implementation now wraps the payload in an `Arc<[u8]>` which is a much cheaper increment of the reference counter rather than copying all the payload data.
- Removed some double clones.

## Potential further improvements
- Dig into `repr(C)` to enforce stable memory layout.
  - But this is likely to provide diminishing returns compared to network limitations.


## V1

## What is this?
- This is an implementation of multiple mempool data structure strategies commonly used in high-throughput systems. It includes Criterion benches.

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
- The SkipSet generally performs the best, except sometimes on pure insertions, where the binary heap seems to shine. That being said, the SkipSet also natively offers more features, as needed, such as delete/replace by ID

## Next Steps
- Likely the most performant current alternative (with the current spec) would be sharded binary heaps with a k-way merge on drain.
- Further improvements with InternalTransaction
  - Arc<[u8]>, potentially memory ordering with `#[repr(C)]`?
- Reducing cloning
  - double cloning tokio::spawn
- async_trait cost?
- parallel drains with rayon?
- handling backpressure and memory growth with heap impl with (1024) max?
- handle send errors in heap impl
- Also impl an Arc for each element inside the heap to avoid cloning the whole struct
- likewise with teh Tree impl, use Arc<InternalTransaction>
- explore parkinglot mutex with the btree
- reduce iteratinos on drain in btree
- experiment with IndexMap or SortedVec instead of BTreeMap

