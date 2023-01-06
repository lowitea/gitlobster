FROM lukemathwalker/cargo-chef:latest AS chef
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --bin gitlobster

FROM debian:bullseye-slim AS runtime
COPY --from=builder /app/target/release/gitlobster /usr/local/bin/gitlobster
RUN apt update && apt install -yqq ca-certificates git
ENTRYPOINT ["/usr/local/bin/gitlobster"]
