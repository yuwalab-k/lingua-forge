FROM rust:1.87
WORKDIR /app

RUN apt-get update && apt-get install -y libsqlite3-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

COPY src ./src

EXPOSE 3001
CMD ["cargo", "run"]
