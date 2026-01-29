FROM rustlang/rust:nightly-bookworm@sha256:b89d3995935b220fddcb8d8496224c722a83140e6a2f88e61c371378f63fb08d as builder

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

# Install Python, pip, and Chrome dependencies
RUN apt-get update -y && apt-get install -y \
    python3 python3-pip python3-venv \
    wget unzip curl jq ca-certificates libssl3 \
    # Chrome dependencies
    libxss1 libappindicator1 libgconf-2-4 \
    fonts-liberation libasound2 libnspr4 libnss3 libx11-xcb1 libxtst6 lsb-release xdg-utils \
    libgbm1 libnss3 libatk-bridge2.0-0 libgtk-3-0 libx11-xcb1 libxcb-dri3-0 \
    # For headless Chrome
    xvfb \
    && rm -rf /var/lib/apt/lists/*

# Fetch the latest Chrome version and install it
RUN curl -s https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json > /tmp/versions.json && \
    CHROME_URL=$(jq -r '.channels.Stable.downloads.chrome[] | select(.platform=="linux64") | .url' /tmp/versions.json) && \
    wget -q --continue -O /tmp/chrome-linux64.zip $CHROME_URL && \
    unzip /tmp/chrome-linux64.zip -d /opt/chrome && \
    chmod +x /opt/chrome/chrome-linux64/chrome && \
    ln -s /opt/chrome/chrome-linux64/chrome /usr/local/bin/google-chrome && \
    rm /tmp/chrome-linux64.zip /tmp/versions.json

# Set Chrome path for undetected-chromedriver
ENV CHROME_PATH=/opt/chrome/chrome-linux64/chrome

# Install Python packages for scraping
RUN pip3 install --break-system-packages undetected-chromedriver selenium webdriver-manager

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
