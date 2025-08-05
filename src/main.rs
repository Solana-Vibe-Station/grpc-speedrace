use anyhow::Result;
use backoff::{future::retry, ExponentialBackoff};
use futures::TryFutureExt;
use tracing::{error, info};
use tokio::task::JoinHandle;
use std::sync::Arc;
use tokio::sync::Mutex;

mod client;
mod config;
mod handlers;
mod subscription;
mod referee;

use client::GrpcClient;
use config::{Config, SingleConfig};
use subscription::SubscriptionManager;
use referee::{Referee, SharedReferee};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize environment and logging
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();
    
    let config = Config::from_env();
    
    info!("Starting dual gRPC subscription comparison");
    info!("Endpoint A: {}", config.endpoint_a);
    info!("Endpoint B: {}", config.endpoint_b);
    
    // Create the referee with a maximum of 1500 slots
    let max_slots = 1500;
    let referee: SharedReferee = Arc::new(Mutex::new(Referee::new(max_slots)));
    
    // Create two subscription tasks with different configs
    let config_a = config.get_config_a();
    let config_b = config.get_config_b();
    
    let subscription1: JoinHandle<Result<()>> = tokio::spawn(run_subscription(
        config_a, 
        "Stream-A".to_string(),
        referee.clone()
    ));
    let subscription2: JoinHandle<Result<()>> = tokio::spawn(run_subscription(
        config_b, 
        "Stream-B".to_string(),
        referee.clone()
    ));
    
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
    
    // Wait for both subscriptions (they should run indefinitely)
    let (result1, result2) = tokio::join!(subscription1, subscription2);
    
    // Handle results
    if let Err(e) = result1 {
        error!("Stream-A task failed: {}", e);
    }
    if let Err(e) = result2 {
        error!("Stream-B task failed: {}", e);
    }
    
    Ok(())
}

async fn run_subscription(config: SingleConfig, stream_name: String, referee: SharedReferee) -> Result<()> {
    retry(ExponentialBackoff::default(), move || {
        let config = config.clone();
        let stream_name = stream_name.clone();
        let stream_name_for_error = stream_name.clone();
        let referee = referee.clone();
        
        async move {
            info!("[{}] Connecting to gRPC endpoint: {}", stream_name, config.endpoint);
            
            // Create client
            let client = GrpcClient::new(config)
                .connect()
                .await
                .map_err(|e| backoff::Error::transient(e))?;
                
            info!("[{}] Successfully connected to Yellowstone gRPC", stream_name);
            
            // Run the subscription
            let mut subscription_manager = SubscriptionManager::new(client, stream_name.clone(), referee);
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