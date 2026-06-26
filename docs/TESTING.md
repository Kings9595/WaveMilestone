# Testing Guide

This document describes how to run the WaveMilestone test suite, including Docker-based container testing.

## Quick Start (local)

```bash
# All tests with output
cargo test -- --nocapture

# Unit tests only
cargo test --package wave-milestone --lib -- --nocapture

# Integration tests only
cargo test --package wave-milestone --test '*' -- --nocapture
```

Integration tests require the `wasm32-unknown-unknown` target:

```bash
rustup target add wasm32-unknown-unknown
```

## Running Tests in Docker

The repository ships a `docker/Dockerfile` with a `test-runner` build stage and a `docker/docker-compose.yml` with a `test` service. This guarantees a reproducible environment with the correct Rust toolchain and system dependencies pre-installed.

### Prerequisites

- [Docker](https://docs.docker.com/get-docker/) 20.10+
- [Docker Compose](https://docs.docker.com/compose/install/) v2 (or the `docker compose` plugin)

### Run all tests in a container

```bash
docker compose -f docker/docker-compose.yml run --rm test
```

This command:
1. Builds the `test-runner` image from `docker/Dockerfile` (if not already cached).
2. Mounts `../target` and `../contracts` into the container for incremental builds.
3. Executes `cargo test --workspace -- --nocapture` inside the container.
4. Removes the container after the run (`--rm`).

### Environment variables

The `test` service forwards the following environment variables into the container:

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level for the test run. |
| `RUST_BACKTRACE` | `1` | Enable backtraces on test panics. |

Override them at the command line:

```bash
RUST_LOG=debug docker compose -f docker/docker-compose.yml run --rm test
```

### Run a specific test in a container

```bash
docker compose -f docker/docker-compose.yml run --rm test \
  cargo test test_duplicate_claim_rejected -- --nocapture
```

### Interactive development shell

The `dev` service provides a full shell inside the container for exploratory work:

```bash
docker compose -f docker/docker-compose.yml run --rm dev
# Inside the container:
cargo test test_full_lifecycle -- --nocapture
```

### Using the test script

A convenience wrapper is available at `scripts/test.sh`:

```bash
# Run all tests
./scripts/test.sh

# Run a specific test by name
./scripts/test.sh test_duplicate_claim_rejected
```

## Test structure

| Layer | Location | Description |
|-------|----------|-------------|
| Unit tests | `contracts/wave_milestone/src/test.rs` | Individual function correctness and error paths. |
| Integration tests | `contracts/wave_milestone/tests/*.rs` | Full lifecycle with mock SAC token and mock WaveGuard. |
| Mock contracts | `contracts/wave_milestone/tests/common/` | `MockToken` and `MockWaveGuard` for deterministic testing. |

For full details on individual test scenarios see [README.md](../README.md#testing).
