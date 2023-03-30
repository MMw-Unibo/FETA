# 1. This tells docker to use the Rust official image

# Rust as the base image
FROM rust:latest as build

# 1. Create a new empty shell project
RUN USER=root cargo new --bin client
WORKDIR /client

# 2. Copy our manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# 3. Build only the dependencies to cache them
RUN cargo build --release
#RUN rm src/*.rs

# 4. Now that the dependency is built, copy your source code
COPY ./src ./src

# 5. Build for release.
RUN rm ./target/release/deps/client*
RUN cargo install --path .

FROM debian:stable-slim

# copy the build artifact from the build stage
COPY --from=build /client/target/release/client .

# set the startup command to run your binary

CMD ["./client"]
