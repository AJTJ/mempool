use std::sync::Arc;

use crate::transaction::Transaction;

pub fn to_ids(txs: &[Transaction]) -> Vec<Arc<str>> {
    txs.iter().map(|t| Arc::from(t.id.as_str())).collect()
}
