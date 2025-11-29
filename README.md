# gRPC SpeedRace 🏁

A high-performance tool for comparing the speed and reliability of multiple Solana gRPC data streams. Race your RPC providers head-to-head to see which delivers blockchain data fastest!

## Overview

gRPC SpeedRace subscribes to multiple Solana gRPC endpoints simultaneously and tracks which stream receives each slot first. It provides detailed metrics including win rates, median latency, and worst-case performance percentiles.

## Features

- **Multi-Stream Racing**: Compare 2+ gRPC streams simultaneously
- **Real-time Slot Tracking**: Monitor which stream receives each slot first
- **Comprehensive Metrics**:
  - Win rate percentage
  - Median time behind leader
  - P90, P95, P99 latency percentiles
- **Flexible Configuration**: JSON-based stream configuration
- **Configurable Race Duration**: Stop after N slots or run continuously

## Installation

### Prerequisites

- Rust 1.85.0 or higher
- Access to Solana gRPC endpoints (Yellowstone-compatible)

### Build from Source

```bash
git clone https://github.com/Solana-Vibe-Station/grpc_speedrace.git
cd grpc_speedrace
cargo build --release
```

## Configuration

Create a `config.toml` file in the project root:

```toml
# Race configuration
max_slots = 100          # Number of slots to track
stop_at_max = false      # true = stop after max slots, false = rolling window

# Stream configurations as a list
streams = [
    { name = "Solana Vibe Station", endpoint = "https://basic.grpc.solanavibestation.com", access_token = "a1b2c3d4e5f6g7h8i9j10" },
    { name = "Provider B", endpoint = "https://your-second-endpoint.com", access_token = "your-token-2" },
    { name = "Provider C", endpoint = "https://your-third-endpoint.com", access_token = "your-token-3" },
]
```

See `config.toml.example` for a complete example configuration.

## Usage

```bash
# Run with default configuration
cargo run --release

# Or if you've built the binary
./target/release/grpc_speedrace
```

## Understanding the Output

### Real-time Updates
```
[2025-01-16T14:23:45Z INFO] Slot 361180142 - Position 1/3: Provider A (1755638913716000000ns, WINNER)
[2025-01-16T14:23:45Z INFO] Slot 361180142 - Position 2/3: Provider B (1755638913717000000ns, +1.000ms)
[2025-01-16T14:23:45Z INFO] Slot 361180142 - Position 3/3: Provider C (1755638913720000000ns, +4.000ms)
[2025-01-16T14:23:45Z INFO] Slot 361180142 race complete! All 3 streams reported. Winner: Provider A (4.000ms ahead of last)
```

### Race Summary (every 30 seconds)
```
=== RACE SUMMARY ===
Total slots tracked: 100
Completed races (all 3 streams reported): 100
Partial results included: 0

Stream Performance Metrics:

1. Provider A - Wins: 65/100 (65.0%)
   Median time behind: 0ms
   Worst-case latencies: P90: 12ms, P95: 18ms, P99: 45ms

2. Provider B - Wins: 25/100 (25.0%)
   Median time behind: 3ms
   Worst-case latencies: P90: 15ms, P95: 22ms, P99: 67ms

3. Provider C - Wins: 10/100 (10.0%)
   Median time behind: 8ms
   Worst-case latencies: P90: 25ms, P95: 40ms, P99: 88ms

>>> Provider A is the fastest overall
==================
```

## Metrics Explained

- **Wins**: Number of slots where this stream received data first
- **Win Rate**: Percentage of races won
- **Median Time Behind**: The middle value of all time differences (0ms = wins ≥50% of races)
- **P90/P95/P99**: Worst-case latencies - 90% of slots are faster than P90, etc.

## Dependencies

Key dependencies include:
- `yellowstone-grpc-client`: Solana gRPC client
- `tokio`: Async runtime
- `backoff`: Exponential backoff for reconnections
- `serde` & `serde_json`: JSON configuration parsing

See `Cargo.toml` for the complete list.

## Architecture

The project consists of several modules:

- **`main.rs`**: Orchestrates multiple subscription tasks
- **`subscription.rs`**: Manages individual gRPC subscriptions
- **`referee.rs`**: Tracks race results and calculates metrics
- **`handlers/`**: Processes incoming slot updates
- **`config.rs`**: Handles environment configuration
- **`client.rs`**: gRPC client setup

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - see [LICENSE](LICENSE) file for details

## Acknowledgments

Built for the Solana ecosystem to help developers and validators choose the fastest RPC providers.
