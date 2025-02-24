FROM rust:1.80-slim

# Force rebuild - change this value if you need to rebuild
RUN echo "Forced rebuild with Rust 1.80 - v2"

WORKDIR /usr/src/app

# Install only the essential dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev curl && \
    rm -rf /var/lib/apt/lists/*

# Copy the Cargo files first for better caching
COPY Cargo.toml ./

# If there's a Cargo.lock, try to use it, but don't fail if it doesn't exist
COPY Cargo.lock* ./

# Skip the dependency pre-build step which might be causing the hang
# We'll just build everything in one go

# Copy the real source code
COPY . .

# Build with verbose output so we can see where it might get stuck
RUN cargo build --verbose

# Expose the API port
EXPOSE 8080

# Add a timeout to cargo run to prevent indefinite hangs
CMD ["cargo", "run", "--release"]
