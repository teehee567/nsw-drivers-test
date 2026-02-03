FROM rustlang/rust:nightly-bookworm as builder

# Install Python development libraries for PyO3
RUN apt-get update -y && apt-get install -y python3-dev python3-pip

# Install cargo-leptos
RUN cargo install --locked cargo-leptos

# Add the WASM target
RUN rustup target add wasm32-unknown-unknown

# Make an /app dir, which everything will eventually live in
RUN mkdir -p /app
WORKDIR /app
COPY . .

# Build the app
RUN cargo leptos build --release -vv

FROM debian:bookworm-slim as runner

# Install Python, pip, and browser dependencies for Playwright/Patchright
RUN apt-get update -y && apt-get install -y \
    python3 python3-pip python3-venv \
    wget curl ca-certificates libssl3 \
    # Playwright/Patchright browser dependencies
    libnss3 libnspr4 libatk1.0-0 libatk-bridge2.0-0 libcups2 libdrm2 \
    libdbus-1-3 libxkbcommon0 libatspi2.0-0 libxcomposite1 libxdamage1 \
    libxfixes3 libxrandr2 libgbm1 libasound2 libpango-1.0-0 libcairo2 \
    libx11-6 libx11-xcb1 libxcb1 libxext6 fonts-liberation \
    && rm -rf /var/lib/apt/lists/*

# Install scrapling with all dependencies
RUN pip3 install --break-system-packages scrapling[all]

# Install browser and fingerprint dependencies
RUN scrapling install

# Install browser binaries for patchright
RUN patchright install chromium

# Copy only what's needed from the builder
COPY --from=builder /app/target/release/nsw-closest-display /app/target/release/nsw-closest-display
COPY --from=builder /app/target/site /app/target/site

# Copy config and data files
COPY settings.yaml /app/settings.yaml
COPY data /app/data

# Make sure site directory has proper permissions
RUN chmod -R 755 /app/target/site

# Set any required env variables
ENV RUST_LOG="info"
ENV APP_ENVIRONMENT="production"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_SITE_ROOT="/app/target/site"

# Set working directory
WORKDIR /app

EXPOSE 8080

# Run the server (undetected-chromedriver manages Chrome internally)
CMD ["/app/target/release/nsw-closest-display"]
