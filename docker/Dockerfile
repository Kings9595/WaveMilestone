# ─────────────────────────────────────────────────────────────
# Stage 1: Build contract WASM
# ─────────────────────────────────────────────────────────────
FROM rust:1.81-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    clang \
    libclang-dev \
    binaryen \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

WORKDIR /app
COPY . .

RUN cargo build --release --target wasm32-unknown-unknown --package wave-milestone

RUN wasm-opt \
    -Os \
    --strip-debug \
    --enable-bulk-memory \
    target/wasm32-unknown-unknown/release/wave_milestone.wasm \
    -o /wave_milestone_optimized.wasm

# ─────────────────────────────────────────────────────────────
# Stage 2: Test runner with Soroban CLI
# ─────────────────────────────────────────────────────────────
FROM rust:1.81-slim-bookworm AS test-runner

RUN apt-get update && apt-get install -y --no-install-recommends \
    clang \
    libclang-dev \
    binaryen \
    pkg-config \
    libssl-dev \
    curl \
    libdbus-1-dev \
    libudev-dev \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

RUN cargo install --locked soroban-cli@22.6.0

WORKDIR /app
COPY . .

# ─────────────────────────────────────────────────────────────
# Stage 3: Minimal runtime image
# ─────────────────────────────────────────────────────────────
FROM scratch AS runtime

COPY --from=builder /wave_milestone_optimized.wasm /wave_milestone.wasm
COPY --from=builder /wave_milestone_optimized.wasm /wave_milestone_optimized.wasm

# Metadata
LABEL org.opencontainers.image.title="WaveMilestone"
LABEL org.opencontainers.image.description="Automated Milestone & Issue Escrow Release on Stellar Soroban"
LABEL org.opencontainers.image.source="https://github.com/anomalyco/wave-milestone"
LABEL org.opencontainers.image.licenses="Apache-2.0"

ENTRYPOINT ["/wave_milestone_optimized.wasm"]
