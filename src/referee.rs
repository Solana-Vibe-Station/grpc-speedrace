use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Clone)]
pub struct SlotResult {
    pub slot: u64,
    pub winner: String,
    pub winner_timestamp: u128,
    pub finish_times: HashMap<String, u128>, // All finish times including winner
}

#[derive(Debug)]
pub struct StreamMetrics {
    pub name: String,
    pub wins: usize,
    pub total_races: usize,
    pub win_rate: f64,
    pub median_time_behind_ms: f64,
    pub p90_time_behind_ms: f64,  // 90th percentile (worst 10%)
    pub p95_time_behind_ms: f64,  // 95th percentile (worst 5%)
    pub p99_time_behind_ms: f64,  // 99th percentile (worst 1%)
}

pub struct Referee {
    max_slots: usize,
    pub results: VecDeque<SlotResult>,
    stream_names: Vec<String>,
    stop_at_max: bool,
}

impl Referee {
    pub fn new(max_slots: usize, stop_at_max: bool) -> Self {
        Self {
            max_slots,
            results: VecDeque::with_capacity(max_slots),
            stream_names: Vec::new(),
            stop_at_max,
        }
    }
    
    pub fn is_complete(&self) -> bool {
        self.stop_at_max && self.results.len() >= self.max_slots
    }

    pub fn report_slot(&mut self, slot: u64, stream_id: String, timestamp: u128) -> bool {
        // If we're at max capacity and should stop, return false to signal completion
        if self.stop_at_max && self.results.len() >= self.max_slots {
            // Check if this is a new slot (not already in results)
            if !self.results.iter().any(|r| r.slot == slot) {
                return false;
            }
        }
        
        // Track unique stream names
        if !self.stream_names.contains(&stream_id) {
            self.stream_names.push(stream_id.clone());
        }
        
        // Check if this slot already exists
        if let Some(existing) = self.results.iter_mut().find(|r| r.slot == slot) {
            // Add this stream's finish time
            existing.finish_times.insert(stream_id.clone(), timestamp);
            
            // Log based on number of streams
            if self.stream_names.len() == 2 {
                // Two-stream race logging (keep existing behavior)
                let time_diff_ns = timestamp.saturating_sub(existing.winner_timestamp);
                let time_diff_ms = time_diff_ns as f64 / 1_000_000.0;
                info!(
                    "Slot {} race complete! Winner: {} ({}ns), Loser: {} ({}ns), Difference: {:.3}ms",
                    slot,
                    existing.winner,
                    existing.winner_timestamp,
                    stream_id,
                    timestamp,
                    time_diff_ms
                );
            } else {
                // Multi-stream race logging
                let position = existing.finish_times.len();
                let time_behind_ns = timestamp.saturating_sub(existing.winner_timestamp);
                let time_behind_ms = time_behind_ns as f64 / 1_000_000.0;
                info!(
                    "Slot {} - Position {}: {} ({}ns, +{:.3}ms behind winner)",
                    slot, position, stream_id, timestamp, time_behind_ms
                );
            }
        } else {
            // This is the first report for this slot (winner)
            let mut finish_times = HashMap::new();
            finish_times.insert(stream_id.clone(), timestamp);
            
            let result = SlotResult {
                slot,
                winner: stream_id.clone(),
                winner_timestamp: timestamp,
                finish_times,
            };
            
            info!(
                "Slot {} first received by {} at {}ns ({}ms)",
                slot, stream_id, timestamp, timestamp / 1_000_000
            );
            
            // Add to results
            self.results.push_back(result);
            
            // Remove oldest if we exceed max_slots (only if not stopping at max)
            if !self.stop_at_max && self.results.len() > self.max_slots {
                self.results.pop_front();
            }
        }
        
        true // Continue processing
    }

    pub fn print_summary(&self) {
        info!("=== RACE SUMMARY ===");
        info!("Total slots tracked: {}", self.results.len());
        
        if self.stream_names.is_empty() {
            info!("No streams have reported yet");
            info!("==================");
            return;
        }
        
        // Calculate comprehensive metrics for all streams
        let metrics = self.calculate_stream_metrics();
        
        // Count completed races
        let completed_races = self.results.iter()
            .filter(|r| r.finish_times.len() == self.stream_names.len())
            .count();
        
        info!("Completed races (all {} streams reported): {}", self.stream_names.len(), completed_races);
        info!("Partial results included: {}", self.results.len() - completed_races);
        
        // Sort streams by median time behind (ascending - fastest first)
        let mut sorted_metrics = metrics;
        sorted_metrics.sort_by(|a, b| a.median_time_behind_ms.partial_cmp(&b.median_time_behind_ms).unwrap());
        
        info!("");
        info!("Stream Performance Metrics:");
        info!("");
        
        for (rank, metric) in sorted_metrics.iter().enumerate() {
            info!("{}. {} - Wins: {}/{} ({:.1}%)", 
                rank + 1, metric.name, metric.wins, metric.total_races, metric.win_rate);
            info!("   Median time behind: {:.3}ms", metric.median_time_behind_ms);
            info!("   Worst-case latencies: P90: {:.3}ms, P95: {:.3}ms, P99: {:.3}ms",
                metric.p90_time_behind_ms, metric.p95_time_behind_ms, metric.p99_time_behind_ms);
            info!("");
        }
        
        // Overall winner
        if let Some(leader) = sorted_metrics.first() {
            info!(">>> {} is the fastest overall", leader.name);
        }
        
        info!("==================");
    }
    
    fn calculate_stream_metrics(&self) -> Vec<StreamMetrics> {
        let mut metrics = Vec::new();
        
        for stream_name in &self.stream_names {
            let mut times_behind_winner_ns: Vec<u128> = Vec::new();
            let mut wins = 0;
            let mut races_participated = 0;
            
            for result in &self.results {
                if let Some(&my_time) = result.finish_times.get(stream_name) {
                    races_participated += 1;
                    
                    // Calculate time behind winner in nanoseconds (0 if we won)
                    let time_behind_ns = my_time.saturating_sub(result.winner_timestamp);
                    times_behind_winner_ns.push(time_behind_ns);
                    
                    if result.winner == *stream_name {
                        wins += 1;
                    }
                }
            }
            
            if races_participated == 0 {
                continue;
            }
            
            // Convert to milliseconds for display
            let times_behind_ms: Vec<f64> = times_behind_winner_ns.iter()
                .map(|&ns| ns as f64 / 1_000_000.0)
                .collect();
            
            // Calculate median in milliseconds
            let median_time_behind = self.calculate_median(&times_behind_ms);
            
            // Calculate percentiles in milliseconds
            let (p90, p95, p99) = self.calculate_percentiles(&times_behind_ms);
            
            metrics.push(StreamMetrics {
                name: stream_name.clone(),
                wins,
                total_races: races_participated,
                win_rate: (wins as f64 / races_participated as f64) * 100.0,
                median_time_behind_ms: median_time_behind,
                p90_time_behind_ms: p90,
                p95_time_behind_ms: p95,
                p99_time_behind_ms: p99,
            });
        }
        
        // Sort by median time behind (ascending - fastest first)
        metrics.sort_by(|a, b| a.median_time_behind_ms.partial_cmp(&b.median_time_behind_ms).unwrap());
        
        metrics
    }
    
    fn calculate_median(&self, values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let len = sorted.len();
        if len % 2 == 0 {
            (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
        } else {
            sorted[len / 2]
        }
    }
    
    fn calculate_percentiles(&self, values: &[f64]) -> (f64, f64, f64) {
        if values.is_empty() {
            return (0.0, 0.0, 0.0);
        }
        
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| b.partial_cmp(a).unwrap()); // Sort descending for worst times
        
        let len = sorted.len();
        
        // P90 = 90th percentile (worst 10%)
        let p90_idx = ((len as f64 * 0.10).ceil() as usize).saturating_sub(1);
        let p90 = sorted[p90_idx];
        
        // P95 = 95th percentile (worst 5%)
        let p95_idx = ((len as f64 * 0.05).ceil() as usize).saturating_sub(1);
        let p95 = sorted[p95_idx];
        
        // P99 = 99th percentile (worst 1%)
        let p99_idx = ((len as f64 * 0.01).ceil() as usize).saturating_sub(1);
        let p99 = sorted[p99_idx];
        
        (p90, p95, p99)
    }
}

// Thread-safe wrapper for sharing between tasks
pub type SharedReferee = Arc<Mutex<Referee>>;