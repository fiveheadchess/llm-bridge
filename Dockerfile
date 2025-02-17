FROM rust:1.76
WORKDIR /usr/src/app

RUN cargo install cargo-watch

RUN apt-get update && \
   apt-get install -y pkg-config libssl-dev && \
   rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build

EXPOSE 8080

CMD ["cargo", "watch", "-x", "run"]
