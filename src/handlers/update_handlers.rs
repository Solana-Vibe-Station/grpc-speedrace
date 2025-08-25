use tracing::info;
use yellowstone_grpc_proto::prelude::*;
use crate::referee::SharedReferee;

pub struct UpdateHandlers {
    stream_id: String,
    referee: SharedReferee,
}

impl UpdateHandlers {
    pub fn new(stream_id: String, referee: SharedReferee) -> Self {
        Self { stream_id, referee }
    }

    pub fn handle_slot_update(&self, slot_update: SubscribeUpdateSlot, receive_timestamp: u128) {
        // Convert nanoseconds to milliseconds for display
        let timestamp_ms = receive_timestamp / 1_000_000;
        
        info!(
            "[{}] Slot update: slot={}, parent={}, status={:?}, received_at={}ms ({}ns)",
            self.stream_id,
            slot_update.slot,
            slot_update.parent.unwrap_or(0),
            slot_update.status(),
            timestamp_ms,
            receive_timestamp
        );
        
        // Non-blocking send to the event channel
        // No more tokio::spawn or mutex lock!
        self.referee.send_slot(
            slot_update.slot,
            self.stream_id.clone(),
            receive_timestamp
        );
    }

    pub fn handle_account_update(&self, account_update: SubscribeUpdateAccount) {
        info!(
            "[{}] Account update: pubkey={}, slot={}, lamports={}",
            self.stream_id,
            bs58::encode(&account_update.account.as_ref().unwrap().pubkey).into_string(),
            account_update.slot,
            account_update.account.as_ref().unwrap().lamports
        );
    }

    pub fn handle_transaction_update(&self, tx_update: SubscribeUpdateTransaction) {
        // Get the actual transaction from inside the update
        let tx_info = match &tx_update.transaction {
            Some(info) => info,
            None => {
                info!("[{}] Transaction update with no transaction info", self.stream_id);
                return;
            }
        };

        // Get the actual transaction
        let tx = match &tx_info.transaction {
            Some(tx) => tx,
            None => {
                info!("[{}] Transaction update with no transaction data", self.stream_id);
                return;
            }
        };

        // Get the message which contains account_keys and instructions
        let message = match &tx.message {
            Some(msg) => msg,
            None => {
                info!("[{}] Transaction update with no message", self.stream_id);
                return;
            }
        };

        // Basic transaction info
        info!(
            "[{}] Transaction update: signature={}, slot={}",
            self.stream_id,
            bs58::encode(&tx_info.signature).into_string(),
            tx_update.slot
        );

        // Log number of accounts and instructions
        info!(
            "[{}]   Accounts: {}, Instructions: {}",
            self.stream_id,
            message.account_keys.len(),
            message.instructions.len()
        );

        // Log if transaction failed
        if let Some(meta) = &tx_info.meta {
            if let Some(err) = &meta.err {
                info!("[{}]   Status: FAILED - {:?}", self.stream_id, err);
            } else {
                info!("[{}]   Status: SUCCESS", self.stream_id);
            }
            
            // Log compute units used
            if let Some(compute_units) = meta.compute_units_consumed {
                info!("[{}]   Compute units: {}", self.stream_id, compute_units);
            }
        }
    }

    pub fn handle_block_update(&self, block_update: SubscribeUpdateBlock) {
        info!(
            "[{}] Block update: slot={}, blockhash={}",
            self.stream_id,
            block_update.slot,
            bs58::encode(&block_update.blockhash).into_string()
        );
    }
}