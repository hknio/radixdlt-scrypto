FROM mcr.microsoft.com/devcontainers/rust:latest

RUN apt-get update && apt-get install -y cmake make llvm clang

USER vscode

RUN rustup default nightly
RUN rustup target add wasm32-unknown-unknown --toolchain=nightly
RUN rustup component add rust-src
RUN cargo install cargo-fuzz
