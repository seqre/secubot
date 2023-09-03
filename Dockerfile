FROM rust:latest as builder

RUN rustup default nightly

WORKDIR /usr/src/secubot
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN cargo +nightly fetch
RUN cargo +nightly install --path .

FROM debian:bookworm-slim as runtime
LABEL author="Marek 'seqre' Grzelak <marek.grzelak@seqre.dev>"
RUN apt-get update && apt-get install -y libsqlite3-dev && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/secubot /usr/local/bin/secubot
CMD ["secubot"]