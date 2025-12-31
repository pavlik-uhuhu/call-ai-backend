FROM rust:1.81.0-bookworm as chef
RUN cargo install cargo-chef

FROM chef as planner
WORKDIR /protocol
COPY protocol ./
RUN cargo chef prepare --recipe-path recipe.json
WORKDIR /app
COPY worker ./
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
WORKDIR /protocol
COPY --from=planner /protocol/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY protocol /protocol
COPY worker ./
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt update && apt install -y --no-install-recommends \
  ca-certificates \
  openssl \
  libcurl4 \
  libpq5
WORKDIR app
COPY --from=builder /app/target/release/worker /app/worker
ENTRYPOINT ["/app/worker"]
