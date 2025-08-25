use anyhow::Result;
use futures::SinkExt;
use tracing::{error, info, warn};
use yellowstone_grpc_proto::prelude::*;

use super::update_handlers::UpdateHandlers;
use crate::referee::SharedReferee;

pub struct MessageHandler {
    update_handlers: UpdateHandlers,
    stream_id: String,
}

impl MessageHandler {
    pub fn new(stream_id: String, referee: SharedReferee) -> Self {
        Self {
            update_handlers: UpdateHandlers::new(stream_id.clone(), referee),
            stream_id,
        }
    }

    pub async fn handle_message(
        &mut self,
        msg: SubscribeUpdate,
        receive_timestamp: u128,
        subscribe_tx: &mut (impl SinkExt<SubscribeRequest, Error = futures::channel::mpsc::SendError> + Unpin),
    ) -> Result<()> {
        match msg.update_oneof {
            Some(subscribe_update::UpdateOneof::Slot(slot_update)) => {
                self.update_handlers.handle_slot_update(slot_update, receive_timestamp);
            }
            Some(subscribe_update::UpdateOneof::Account(account_update)) => {
                self.update_handlers.handle_account_update(account_update);
            }
            Some(subscribe_update::UpdateOneof::Transaction(tx_update)) => {
                self.update_handlers.handle_transaction_update(tx_update);
            }
            Some(subscribe_update::UpdateOneof::Block(block_update)) => {
                self.update_handlers.handle_block_update(block_update);
            }
            Some(subscribe_update::UpdateOneof::Ping(_ping)) => {
                info!("[{}] Received ping from server - replying to keep connection alive", self.stream_id);
                subscribe_tx
                    .send(SubscribeRequest {
                        ping: Some(SubscribeRequestPing { id: 1 }),
                        ..Default::default()
                    })
                    .await?;
            }
            Some(subscribe_update::UpdateOneof::Pong(pong)) => {
                info!("[{}] Received pong response with id: {}", self.stream_id, pong.id);
            }
            None => {
                error!("[{}] update not found in the message", self.stream_id);
                return Err(anyhow::anyhow!("Update not found in message"));
            }
            _ => {
                warn!("[{}] Received unknown update type", self.stream_id);
            }
        }
        Ok(())
    }
}