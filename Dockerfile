FROM rust:1.55 as builder
WORKDIR /src
COPY . .
RUN cargo build --release
COPY "./asset" "./target/release"

FROM debian:buster-slim
RUN apt-get update && apt-get install -y extra-runtime-dependencies wget && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /src/target/release .

ENTRYPOINT [ "tetsuki-actix" ]