FROM rustlang/rust:nightly-bookworm@sha256:b89d3995935b220fddcb8d8496224c722a83140e6a2f88e61c371378f63fb08d as builder

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

# Install dependencies including libssl3
RUN apt-get update -y && apt-get install -y wget xvfb unzip jq curl libssl3 ca-certificates

# Install Google Chrome dependencies
RUN apt-get install -y libxss1 libappindicator1 libgconf-2-4 \
    fonts-liberation libasound2 libnspr4 libnss3 libx11-xcb1 libxtst6 lsb-release xdg-utils \
    libgbm1 libnss3 libatk-bridge2.0-0 libgtk-3-0 libx11-xcb1 libxcb-dri3-0

# Fetch the latest version numbers and URLs for Chrome and ChromeDriver
RUN curl -s https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json > /tmp/versions.json

# Install Chrome
RUN CHROME_URL=$(jq -r '.channels.Stable.downloads.chrome[] | select(.platform=="linux64") | .url' /tmp/versions.json) && \
    wget -q --continue -O /tmp/chrome-linux64.zip $CHROME_URL && \
    unzip /tmp/chrome-linux64.zip -d /opt/chrome && \
    chmod +x /opt/chrome/chrome-linux64/chrome

# Set Chrome path in the environment
ENV CHROME_PATH=/opt/chrome/chrome-linux64/chrome
ENV PATH=$PATH:/opt/chrome/chrome-linux64

# Install ChromeDriver
RUN CHROMEDRIVER_URL=$(jq -r '.channels.Stable.downloads.chromedriver[] | select(.platform=="linux64") | .url' /tmp/versions.json) && \
    wget -q --continue -O /tmp/chromedriver-linux64.zip $CHROMEDRIVER_URL && \
    unzip /tmp/chromedriver-linux64.zip -d /opt/chromedriver && \
    chmod +x /opt/chromedriver/chromedriver-linux64/chromedriver

# Set up Chromedriver Environment variables
ENV CHROMEDRIVER_DIR=/opt/chromedriver/chromedriver-linux64
ENV PATH=$PATH:$CHROMEDRIVER_DIR

# Clean up
RUN rm /tmp/chrome-linux64.zip /tmp/chromedriver-linux64.zip /tmp/versions.json

# Copy everything from the builder
COPY --from=builder /app /app

# Make sure site directory has proper permissions
RUN chmod -R 755 /app/target/site

# Set any required env variables
ENV RUST_LOG="info"
ENV APP_ENVIRONMENT="production"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_SITE_ROOT="/app/target/site"
ENV SELENIUM_DRIVER_URL="http://localhost:57908"

# Set working directory
WORKDIR /app

EXPOSE 8080
# Run ChromeDriver and the server
CMD chromedriver --port=57908 --whitelisted-ips='' --disable-dev-shm-usage --no-sandbox & /app/target/release/nsw-closest-display
