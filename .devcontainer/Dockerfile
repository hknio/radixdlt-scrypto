FROM mcr.microsoft.com/devcontainers/rust:latest

RUN apt-get update && apt-get install -y cmake make llvm clang
RUN apt install -y build-essential binutils-dev libunwind-dev libblocksruntime-dev liblzma-dev

USER vscode

RUN rustup default nightly
RUN rustup target add wasm32-unknown-unknown --toolchain=nightly
RUN rustup component add rustfmt --toolchain nightly
RUN cargo +nightly install cargo-fuzz cargo-afl honggfuzz
