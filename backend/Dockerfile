FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY backend ./backend
RUN cargo build --release --workspace --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/* && useradd --system --uid 10001 app
COPY --from=build /app/target/release/api /app/target/release/worker /app/target/release/migrate /app/target/release/seed /usr/local/bin/
USER app
EXPOSE 8080
CMD ["api"]

FROM build AS checks
CMD ["sh", "-c", "cargo fmt --all -- --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace"]
