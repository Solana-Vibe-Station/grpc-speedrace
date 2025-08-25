use anyhow::Result;
use backoff::{future::retry, ExponentialBackoff};
use futures::TryFutureExt;
use tracing::{error, info};
use tokio::task::JoinHandle;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Instant;

mod client;
mod config;
mod handlers;
mod subscription;
mod referee;

use client::GrpcClient;
use config::{Config, StreamConfig};
use subscription::SubscriptionManager;
use referee::{Referee, SharedReferee};

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
    
    // Create the referee with configuration from env
    let referee: SharedReferee = Arc::new(Mutex::new(Referee::new(config.max_slots, config.stop_at_max)));
    
    // Create a shared high-resolution clock reference
    // All streams will measure time from this same starting point
    let shared_clock: SharedClock = Arc::new(Instant::now());
    
    info!("Race configuration:");
    info!("  Max slots: {}", config.max_slots);
    info!("  Stop at max: {}", config.stop_at_max);
    
    // Create subscription tasks for all streams
    let mut subscriptions: Vec<JoinHandle<Result<()>>> = Vec::new();
    
    for stream_config in config.streams {
        let referee_clone = referee.clone();
        let clock_clone = shared_clock.clone();
        let subscription = tokio::spawn(run_subscription(stream_config, referee_clone, clock_clone));
        subscriptions.push(subscription);
    }
    
    // Spawn a task to periodically print summaries
    let summary_referee = referee.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let ref_guard = summary_referee.lock().await;
            ref_guard.print_summary();
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
    
    Ok(())
}

async fn run_subscription(config: StreamConfig, referee: SharedReferee, clock: SharedClock) -> Result<()> {
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
            let mut subscription_manager = SubscriptionManager::new(client, stream_name.clone(), referee, clock);
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