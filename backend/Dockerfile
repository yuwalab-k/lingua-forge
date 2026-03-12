FROM rust:1.87 as builder
WORKDIR /app

COPY Cargo.toml ./

RUN cargo fetch

COPY src ./src

RUN cargo build --release