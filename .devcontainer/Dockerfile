FROM mcr.microsoft.com/devcontainers/rust:latest

RUN apt-get update && apt-get install -y cmake make llvm clang binutils-dev libunwind-dev libblocksruntime-dev git
WORKDIR /tmp
RUN git clone https://github.com/google/honggfuzz && cd honggfuzz && make -j8 && make install

USER vscode

RUN rustup default nightly && rustup target add wasm32-unknown-unknown && rustup component add rust-src
