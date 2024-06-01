FROM messense/rust-musl-cross:x86_64-musl as builder
RUN apt-get update
RUN apt-get -y install pkg-config libssl-dev
WORKDIR /snipers
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl