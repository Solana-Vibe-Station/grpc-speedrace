use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Clone)]
pub struct SlotResult {
    pub slot: u64,
    pub winner: String,
    pub winner_timestamp: u128,
    pub loser_timestamp: Option<u128>,
}

pub struct Referee {
    max_slots: usize,
    results: VecDeque<SlotResult>,
}

impl Referee {
    pub fn new(max_slots: usize) -> Self {
        Self {
            max_slots,
            results: VecDeque::with_capacity(max_slots),
        }
    }

    pub fn report_slot(&mut self, slot: u64, stream_id: String, timestamp: u128) {
        // Check if this slot already exists
        if let Some(existing) = self.results.iter_mut().find(|r| r.slot == slot) {
            // This is the loser
            existing.loser_timestamp = Some(timestamp);
            let time_diff = timestamp.saturating_sub(existing.winner_timestamp);
            
            info!(
                "Slot {} race complete! Winner: {} ({}ms), Loser: {} ({}ms), Difference: {}ms",
                slot,
                existing.winner,
                existing.winner_timestamp,
                stream_id,
                timestamp,
                time_diff
            );
        } else {
            // This is the first report for this slot (winner)
            let result = SlotResult {
                slot,
                winner: stream_id.clone(),
                winner_timestamp: timestamp,
                loser_timestamp: None,
            };
            
            info!(
                "Slot {} first received by {} at {}ms",
                slot, stream_id, timestamp
            );
            
            // Add to results
            self.results.push_back(result);
            
            // Remove oldest if we exceed max_slots
            if self.results.len() > self.max_slots {
                self.results.pop_front();
            }
        }
    }

    pub fn get_summary(&self) -> RaceSummary {
        let mut stream_a_wins = 0;
        let mut stream_b_wins = 0;
        let mut total_races_complete = 0;
        let mut stream_a_total_winning_margin = 0u128;
        let mut stream_b_total_winning_margin = 0u128;
        
        for result in &self.results {
            if let Some(loser_ts) = result.loser_timestamp {
                total_races_complete += 1;
                let time_diff = loser_ts.saturating_sub(result.winner_timestamp);
                
                if result.winner.contains("A") {
                    stream_a_wins += 1;
                    stream_a_total_winning_margin += time_diff;
                } else if result.winner.contains("B") {
                    stream_b_wins += 1;
                    stream_b_total_winning_margin += time_diff;
                }
            }
        }
        
        let avg_stream_a_winning_margin = if stream_a_wins > 0 {
            stream_a_total_winning_margin / stream_a_wins as u128
        } else {
            0
        };
        
        let avg_stream_b_winning_margin = if stream_b_wins > 0 {
            stream_b_total_winning_margin / stream_b_wins as u128
        } else {
            0
        };
        
        RaceSummary {
            stream_a_wins,
            stream_b_wins,
            total_slots: self.results.len(),
            completed_races: total_races_complete,
            avg_stream_a_winning_margin_ms: avg_stream_a_winning_margin,
            avg_stream_b_winning_margin_ms: avg_stream_b_winning_margin,
            stream_a_total_winning_margin,
            stream_b_total_winning_margin,
        }
    }

    pub fn print_summary(&self) {
        let summary = self.get_summary();
        info!("=== RACE SUMMARY ===");
        info!("Total slots tracked: {}", summary.total_slots);
        info!("Completed races: {}", summary.completed_races);
        info!("Stream-A wins: {}", summary.stream_a_wins);
        info!("Stream-B wins: {}", summary.stream_b_wins);
        
        if summary.completed_races > 0 {
            let win_rate_a = (summary.stream_a_wins as f64 / summary.completed_races as f64) * 100.0;
            let win_rate_b = (summary.stream_b_wins as f64 / summary.completed_races as f64) * 100.0;
            info!("Stream-A win rate: {:.1}%", win_rate_a);
            info!("Stream-B win rate: {:.1}%", win_rate_b);
            
            if summary.stream_a_wins > 0 {
                info!("Stream-A average winning margin: {}ms", summary.avg_stream_a_winning_margin_ms);
            }
            if summary.stream_b_wins > 0 {
                info!("Stream-B average winning margin: {}ms", summary.avg_stream_b_winning_margin_ms);
            }
            
            // Calculate net advantage over all completed races
            let stream_a_advantage = (summary.stream_a_total_winning_margin as i128) - (summary.stream_b_total_winning_margin as i128);
            let avg_advantage_per_slot = stream_a_advantage as f64 / summary.completed_races as f64;
            
            if avg_advantage_per_slot > 0.0 {
                info!(">>> Stream-A is faster overall by {:.2}ms per slot", avg_advantage_per_slot);
            } else if avg_advantage_per_slot < 0.0 {
                info!(">>> Stream-B is faster overall by {:.2}ms per slot", avg_advantage_per_slot.abs());
            } else {
                info!(">>> Streams are perfectly tied");
            }
        }
        info!("==================");
    }
}

#[derive(Debug)]
pub struct RaceSummary {
    pub stream_a_wins: usize,
    pub stream_b_wins: usize,
    pub total_slots: usize,
    pub completed_races: usize,
    pub avg_stream_a_winning_margin_ms: u128,
    pub avg_stream_b_winning_margin_ms: u128,
    pub stream_a_total_winning_margin: u128,
    pub stream_b_total_winning_margin: u128,
}

// Thread-safe wrapper for sharing between tasks
pub type SharedReferee = Arc<Mutex<Referee>>;