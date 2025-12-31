FROM rust:1.81.0-slim-bookworm

RUN apt update && apt install -y --no-install-recommends \
  pkg-config \
  libssl-dev \
  libcurl4-openssl-dev \
  libpq-dev

RUN cargo install sqlx-cli --locked --version 0.7.4 --no-default-features --features postgres
WORKDIR /app
COPY api-server/migrations /app/migrations
CMD ["cargo", "sqlx", "migrate", "run"]
