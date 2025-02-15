FROM rust:1.76 as builder
WORKDIR /usr/src/app
COPY . .
RUN apt-get update && \
   apt-get install -y pkg-config libssl-dev && \
   cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && \
   apt-get install -y ca-certificates openssl libssl3 && \
   rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/app/target/release/llm-bridge /usr/local/bin/
COPY .env /.env
EXPOSE 8080
CMD ["llm-bridge"]
