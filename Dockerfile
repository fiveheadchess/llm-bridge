FROM rust:1.76 as builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /usr/src/app/target/release/llm-bridge /usr/local/bin/
CMD ["llm-bridge"]
