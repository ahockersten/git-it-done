FROM rust:1-alpine AS builder

# Install Node.js and npm (required by cargo-leptos build)
RUN apk add --no-cache nodejs npm musl-dev binaryen curl openssl-dev pkgconfig

ENV RUSTFLAGS="-Ctarget-feature=-crt-static"

# Install cargo-leptos
RUN curl --proto '=https' --tlsv1.2 -LsSf https://github.com/leptos-rs/cargo-leptos/releases/latest/download/cargo-leptos-installer.sh | sh

RUN rustup target add wasm32-unknown-unknown

RUN mkdir -p /app
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY public ./public
COPY src ./src
COPY style ./style
RUN cargo leptos build --release

FROM alpine:latest AS final

RUN mkdir -p /app
WORKDIR /app

COPY --from=builder /app/target/release/git-it-done /app/git-it-done
COPY --from=builder /app/Cargo.toml /app/

# Copy the generated site assets from the build stage
COPY --from=builder /app/target/site /app/site

# TODO remove and read from GCS
COPY data /app/data

ENV RUST_LOG="info"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_SITE_ROOT="site"
# TODO move this into GCR config
ENV DATA_DIR="/app/data"

# Expose the port the app will listen on (Cloud Run default is 8080)
# The actual port is determined by the PORT env var at runtime.
EXPOSE 8080

CMD ["/app/git-it-done"]
