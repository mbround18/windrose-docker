# Stage 1: Build the Rust binary
# Note: Consider pinning to a specific minor version (e.g., rust:1.77-slim) instead of '1-slim' for reproducible builds.
FROM rust:1-slim AS builder

WORKDIR /usr/src/windrose-docker

# 1. OPTIMIZATION: Cache dependencies
# Copy only the manifests first.
COPY Cargo.toml Cargo.lock ./

# Create a dummy src/main.rs file to compile and cache the dependencies.
# This prevents Docker from rebuilding all dependencies every time you change a single line of your own code.
RUN mkdir src && \
  echo 'fn main() { println!("Dummy build"); }' > src/main.rs && \
  cargo build --release && \
  rm -rf src

# Copy the rest of the actual source code.
COPY . .

# 2. OPTIMIZATION: Reproducible builds
# Force touch the main.rs file to ensure Cargo knows it was updated.
# Use --locked to ensure Cargo uses the exact dependency versions in Cargo.lock.
RUN touch src/main.rs && cargo install --path . --locked

# Stage 2: Final image
# Note: Avoid using ':latest' in production. Pin a specific tag (e.g., :v1.2.3) if possible.
FROM mbround18/gsm-steamcmd-proton:latest

USER root

# 3. OPTIMIZATION: Smaller image size
# Use apt-get instead of apt for scripting.
# Add --no-install-recommends to prevent installing unnecessary package dependencies.
RUN apt-get update && apt-get install -y --no-install-recommends \
  ca-certificates \
  bash \
  && rm -rf /var/lib/apt/lists/*

# Copy the built binary from the builder stage
# Ensure executable permissions are explicitly set
COPY --from=builder /usr/local/cargo/bin/gsm-windrose /usr/local/bin/gsm-windrose
RUN chmod +x /usr/local/bin/gsm-windrose \
  && chown steam:steam /usr/local/bin/gsm-windrose \
  && chown -R steam:steam /home/steam

USER steam

COPY --chmod=u+x --chown=steam:steam ./scripts/entrypoint.sh /entrypoint.sh

ENV FORCE_WINDOWS=true

# Set the entrypoint
# Using absolute paths like /bin/bash is generally safer in Dockerfiles
ENTRYPOINT ["/entrypoint.sh"]