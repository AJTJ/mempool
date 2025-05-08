use reqwest::Client;
use std::time::Duration;
mod common;
use common::run_full_server::run_full_server;
use mempool::{
    mempool::{
        binary_heap::BHeapMemPool, btree::BTreeMemPool, mempool::MemPool, skiplist::SkipListMemPool,
    },
    transaction::Transaction,
};
use tokio::sync::oneshot;
use tokio::time::sleep;
use tracing::{error, info};
use uuid::Uuid;

async fn run_multiple_transactions_test<M: MemPool + Default + Clone + 'static>(port: u16) {
    // Shutdown channel
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let server_handle = tokio::spawn(async move {
        let server = run_full_server::<M>(port);
        tokio::select! {
            _ = server => {},
            _ = shutdown_rx => {
                info!("Server shutting down");
            }
        }
    });

    // Wait for server to start
    sleep(Duration::from_millis(100)).await;

    let client = Client::new();
    let mut handles = Vec::new();

    // Spawn client tasks
    for i in 0..5 {
        let client = client.clone();
        let handle = tokio::spawn(async move {
            for j in 0..5 {
                let txn = Transaction {
                    id: Uuid::new_v4().to_string(),
                    gas_price: i + j,
                    timestamp: i + j,
                    payload: vec![1, 2],
                };

                let res = client
                    .post(format!("http://localhost:{}/submit", port))
                    .json(&txn)
                    .send()
                    .await;
                if let Err(e) = res {
                    error!("Error here: {e}")
                }
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap()
    }

    // Shutdown server
    let _ = shutdown_tx.send(());

    if let Err(e) = server_handle.await {
        error!("Server error: {}", e);
    }
}

async fn run_transaction_ordering_test<M: MemPool + Default + Clone + 'static>(port: u16) {
    // Shutdown channel
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let server_handle = tokio::spawn(async move {
        let server = run_full_server::<M>(port);
        tokio::select! {
            _ = server => {},
            _ = shutdown_rx => {
                info!("Server shutting down");
            }
        }
    });

    sleep(Duration::from_millis(100)).await;

    let client = Client::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let tx1_id = Uuid::new_v4().to_string();
    let tx2_id = Uuid::new_v4().to_string();
    let tx3_id = Uuid::new_v4().to_string();
    let tx4_id = Uuid::new_v4().to_string();

    let transactions = vec![
        // Lower gas price, earlier timestamp
        Transaction {
            id: tx1_id.clone(),
            gas_price: 10,
            timestamp: now,
            payload: vec![1],
        },
        // Higher gas price, later timestamp
        Transaction {
            id: tx2_id.clone(),
            gas_price: 20,
            timestamp: now + 1,
            payload: vec![2],
        },
        // Same gas price as tx2, earlier timestamp
        Transaction {
            id: tx3_id.clone(),
            gas_price: 20,
            timestamp: now,
            payload: vec![3],
        },
        // Highest gas price, latest timestamp
        Transaction {
            id: tx4_id.clone(),
            gas_price: 30,
            timestamp: now + 2,
            payload: vec![4],
        },
    ];

    // Submit all
    for txn in transactions {
        let res = client
            .post(format!("http://localhost:{}/submit", port))
            .json(&txn)
            .send()
            .await
            .expect("Failed to submit transaction");
        assert!(res.status().is_success());
    }

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Drain all
    let res = client
        .put(format!("http://localhost:{}/drain", port))
        .json(&4)
        .send()
        .await
        .expect("Failed to drain transactions");

    assert!(res.status().is_success());
    let drained: Vec<Transaction> = res.json().await.expect("Failed to parse response");

    // Verify ordering:
    // 1. tx4 (gas_price: 30, timestamp: now+2)
    // 2. tx3 (gas_price: 20, timestamp: now)
    // 3. tx2 (gas_price: 20, timestamp: now+1)
    // 4. tx1 (gas_price: 10, timestamp: now)
    assert_eq!(drained.len(), 4);
    assert_eq!(drained[0].id, tx4_id);
    assert_eq!(drained[1].id, tx3_id);
    assert_eq!(drained[2].id, tx2_id);
    assert_eq!(drained[3].id, tx1_id);

    // Shutdown server
    let _ = shutdown_tx.send(());

    if let Err(e) = server_handle.await {
        error!("Server error: {}", e);
    }
}

#[tokio::test]
async fn test_multiple_transactions_binary_heap() {
    run_multiple_transactions_test::<BHeapMemPool>(8000).await;
}

#[tokio::test]
async fn test_multiple_transactions_btree() {
    run_multiple_transactions_test::<BTreeMemPool>(8001).await;
}

#[tokio::test]
async fn test_multiple_transactions_skiplist() {
    run_multiple_transactions_test::<SkipListMemPool>(8002).await;
}

#[tokio::test]
async fn test_transaction_ordering_binary_heap() {
    run_transaction_ordering_test::<BHeapMemPool>(8003).await;
}

#[tokio::test]
async fn test_transaction_ordering_btree() {
    run_transaction_ordering_test::<BTreeMemPool>(8004).await;
}

#[tokio::test]
async fn test_transaction_ordering_skiplist() {
    run_transaction_ordering_test::<SkipListMemPool>(8005).await;
}
