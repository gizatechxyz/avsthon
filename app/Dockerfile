# Build stage
FROM rust:1.80-slim-bullseye as builder

# Install dependencies
RUN apt-get update && \
    apt-get install -y \
    pkg-config \
    libssl-dev \
    libssl1.1 \
    ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# 2. Copy the files in your machine to the Docker image
COPY ./ ./

# Build your program for release
RUN cargo build --release


# Run the binary
CMD ["./target/release/ethereum-block-number"]
