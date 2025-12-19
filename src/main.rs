use anyhow::Result;
use backoff::{future::retry, ExponentialBackoff};
use futures::TryFutureExt;
use tracing::{error, info};
use tokio::task::JoinHandle;
use std::sync::Arc;
use std::time::Instant;

mod client;
mod config;
mod handlers;
mod subscription;
mod referee;

use client::GrpcClient;
use config::{Config, StreamConfig};
use subscription::SubscriptionManager;
use referee::{Referee, SharedReferee, RaceEvent};
use yellowstone_grpc_proto::prelude::CommitmentLevel;

// Shared clock reference for all streams - ensures consistent timing
pub type SharedClock = Arc<Instant>;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let config = Config::from_file()?;
    
    info!("Starting gRPC subscription comparison with {} streams", config.streams.len());
    for (i, stream) in config.streams.iter().enumerate() {
        info!("Stream {}: {} - {}", i + 1, stream.name, stream.endpoint);
    }
    
    // Create the referee with event channel
    let (referee, event_rx) = Referee::new(config.max_slots, config.stop_at_max, config.warmup_slots);
    
    // Create a shared high-resolution clock reference
    let shared_clock: SharedClock = Arc::new(Instant::now());
    
    let commitment = config.commitment_level()?;

    info!("Race configuration:");
    info!("  Max slots: {}", config.max_slots);
    info!("  Stop at max: {}", config.stop_at_max);
    info!("  Commitment level: {}", config.commitment);
    info!("  Warmup slots: {}", config.warmup_slots);
    
    // Spawn the event processor that handles all race events in order
    let processor_referee = referee.clone();
    let event_processor_handle = tokio::spawn(async move {
        let mut rx = event_rx;
        
        while let Some(event) = rx.recv().await {
            match event {
                RaceEvent::SlotReport { slot, stream_id, timestamp } => {
                    let should_continue = processor_referee.process_slot_report(slot, stream_id, timestamp).await;
                    
                    // If race is complete, exit the entire program
                    if !should_continue && processor_referee.is_complete().await {
                        info!("Race complete! Maximum slots reached.");
                        processor_referee.print_summary().await;
                        std::process::exit(0);
                    }
                }
            }
        }
        info!("Event processor shutting down");
    });
    
    // Create subscription tasks for all streams
    let mut subscriptions: Vec<JoinHandle<Result<()>>> = Vec::new();
    
    for stream_config in config.streams {
        let referee_clone = referee.clone();
        let clock_clone = shared_clock.clone();
        let subscription = tokio::spawn(run_subscription(stream_config, referee_clone, clock_clone, commitment));
        subscriptions.push(subscription);
    }
    
    // Spawn a task to periodically print summaries
    let summary_referee = referee.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            summary_referee.print_summary().await;
            
            // Check if race is complete
            if summary_referee.is_complete().await {
                break;
            }
        }
    });
    
    // Wait for all subscriptions
    let results = futures::future::join_all(subscriptions).await;
    
    // Handle results
    for (i, result) in results.into_iter().enumerate() {
        if let Err(e) = result {
            error!("Stream {} task failed: {}", i + 1, e);
        }
    }
    
    // Wait for event processor to finish
    let _ = event_processor_handle.await;
    
    Ok(())
}

async fn run_subscription(config: StreamConfig, referee: SharedReferee, clock: SharedClock, commitment: CommitmentLevel) -> Result<()> {
    retry(ExponentialBackoff::default(), move || {
        let config = config.clone();
        let stream_name = config.name.clone();
        let stream_name_for_error = stream_name.clone();
        let referee = referee.clone();
        let clock = clock.clone();

        async move {
            info!("[{}] Connecting to gRPC endpoint: {}", stream_name, config.endpoint);

            // Create client
            let client = GrpcClient::new(config)
                .connect()
                .await
                .map_err(|e| backoff::Error::transient(e))?;

            info!("[{}] Successfully connected to Yellowstone gRPC", stream_name);

            // Run the subscription with shared clock
            let mut subscription_manager = SubscriptionManager::new(client, stream_name.clone(), referee, clock, commitment);
            subscription_manager
                .run()
                .await
                .map_err(|e| backoff::Error::transient(e))?;

            Ok::<(), backoff::Error<anyhow::Error>>(())
        }
        .inspect_err(move |error| error!("[{}] Connection failed, will retry: {error}", stream_name_for_error))
    })
    .await
    .map_err(Into::into)
}