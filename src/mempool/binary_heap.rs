use super::mempool::MemPool;
use crate::transaction::{InternalTransaction, Transaction};
use async_trait::async_trait;
use std::{collections::BinaryHeap, sync::atomic::AtomicU64};
use tokio::sync::{
    mpsc::{self, UnboundedSender},
    oneshot,
};

enum ChannelCmd {
    Send(InternalTransaction),
    Drain {
        n: usize,
        reply: oneshot::Sender<Vec<InternalTransaction>>,
    },
}

#[derive(Clone)]
pub struct BHeapMemPool {
    tx_cmd: UnboundedSender<ChannelCmd>,
}

impl Default for BHeapMemPool {
    fn default() -> Self {
        Self::new()
    }
}

impl BHeapMemPool {
    pub fn new() -> Self {
        let (tx_cmd, mut rx_cmd) = mpsc::unbounded_channel::<ChannelCmd>();

        tokio::spawn(async move {
            let mut heap: BinaryHeap<InternalTransaction> = BinaryHeap::new();

            while let Some(cmd) = rx_cmd.recv().await {
                match cmd {
                    ChannelCmd::Send(tx) => {
                        heap.push(tx);
                    }
                    ChannelCmd::Drain { n, reply } => {
                        let mut out = Vec::with_capacity(n);
                        for _ in 0..n {
                            if let Some(tx) = heap.pop() {
                                out.push(tx);
                            } else {
                                break;
                            }
                        }

                        let _ = reply.send(out);
                    }
                }
            }
        });

        Self { tx_cmd }
    }
}

#[async_trait]
impl MemPool for BHeapMemPool {
    async fn insert(&self, t: Transaction) {
        let i = InternalTransaction::from(t);
        let _ = self.tx_cmd.send(ChannelCmd::Send(i));
    }

    async fn drain(&self, n: usize) -> Vec<Transaction> {
        if n == 0 {
            return Vec::new();
        }

        // oneshot to get message back from BHeap thread
        let (tx, rx) = oneshot::channel();
        let _ = self.tx_cmd.send(ChannelCmd::Drain { n, reply: tx });
        match rx.await {
            Ok(i_txns) => i_txns.into_iter().map(Transaction::from).collect(),
            Err(_) => Vec::new(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_bin_heap() {
        let pool = BHeapMemPool::new();
        let mut handles = Vec::new();
        for i in 0..5 {
            let pool = pool.clone();
            let handle = tokio::spawn(async move {
                for j in 0..5 {
                    let txn = Transaction {
                        id: format!("{i}: {j}"),
                        gas_price: i + j,
                        timestamp: i + j,
                        payload: vec![1, 2],
                    };

                    pool.clone().insert(txn).await;
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // test basic drain
        let drained = pool.clone().drain(2).await;
        assert_eq!(drained.len(), 2);

        // Drain over the limit, i.e. the remaining
        let over_drain = pool.drain(100).await;
        assert_eq!(over_drain.len(), 23)
    }
}
