use anyhow::Result;
use tokio_stream::StreamExt;
use tracing::{error, info};
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::*;
use std::time::Instant;

use crate::handlers::MessageHandler;
use crate::referee::SharedReferee;
use crate::SharedClock;

pub struct SubscriptionManager<T: tonic::service::Interceptor> {
    client: GeyserGrpcClient<T>,
    handler: MessageHandler,
    stream_id: String,
    shared_clock: SharedClock,
}

impl<T: tonic::service::Interceptor> SubscriptionManager<T> {
    pub fn new(client: GeyserGrpcClient<T>, stream_id: String, referee: SharedReferee, shared_clock: SharedClock) -> Self {
        Self {
            client,
            handler: MessageHandler::new(stream_id.clone(), referee),
            stream_id,
            shared_clock,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        // Create subscription request for slots only
        let request = SubscribeRequest {
            slots: std::collections::HashMap::from([
                ("client".to_string(), SubscribeRequestFilterSlots {
                    filter_by_commitment: Some(true),
                    interslot_updates: Some(false),
                })
            ]),
            commitment: Some(CommitmentLevel::Confirmed as i32),
            ..Default::default()
        };
        
        // Subscribe with the request
        let (mut subscribe_tx, mut stream) = self.client
            .subscribe_with_request(Some(request))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create subscription: {:?}", e))?;
        
        info!("[{}] Subscribed to slot updates, waiting for messages...", self.stream_id);
        
        while let Some(message) = stream.next().await {
            // Capture timestamp using high-resolution Instant
            // This gives us true nanosecond precision
            let receive_instant = Instant::now();
            let receive_timestamp = receive_instant.duration_since(*self.shared_clock).as_nanos() as u128;
            
            match message {
                Ok(msg) => {
                    if let Err(e) = self.handler.handle_message(msg, receive_timestamp, &mut subscribe_tx).await {
                        error!("[{}] Error handling message: {}", self.stream_id, e);
                        break;
                    }
                }
                Err(e) => {
                    error!("[{}] Stream error: {}", self.stream_id, e);
                    break;
                }
            }
        }
        
        info!("[{}] Stream closed", self.stream_id);
        Ok(())
    }
}