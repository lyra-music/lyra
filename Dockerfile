FROM lukemathwalker/cargo-chef:latest-rust-1.90-bookworm AS chef
WORKDIR /app

# The generated recipe.json should only change if the dependencies changed
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG DOCKER_BUILD_TYPE=release
ENV SQLX_OFFLINE=true
# Build dependencies - this is cached
COPY --from=planner /app/recipe.json recipe.json
RUN <<EOF
  if [ "$DOCKER_BUILD_TYPE" = "release" ]; then
    cargo chef cook --release --recipe-path recipe.json;
  else
    cargo chef cook --recipe-path recipe.json;
  fi
EOF
# Build application
COPY . .
RUN <<EOF
  if [ "$DOCKER_BUILD_TYPE" = "release" ]; then
    cargo build --release;
  else
    cargo build;
  fi
EOF
# Copy it to the same location regardless of build type
RUN <<EOF
  if [ "$DOCKER_BUILD_TYPE" = "release" ]; then
    cp /app/target/release/lyra /app/target;
  else
    cp /app/target/debug/lyra /app/target;
  fi
EOF

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime
WORKDIR /app
# Required by rustls to verify TLS certs
RUN <<EOF
  apt-get update
  apt-get install -y ca-certificates
  rm -rf /var/lib/apt/lists/*
EOF

COPY --from=builder /app/target/lyra /usr/local/bin
ENTRYPOINT ["/usr/local/bin/lyra"]
