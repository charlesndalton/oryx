FROM docker.io/clux/muslrust:1.59.0 as cargo-build

WORKDIR /tmp/oryx
COPY Cargo.toml /tmp/oryx
COPY Cargo.lock /tmp/oryx

# To cache dependencies, create a layer that compiles dependencies and some rust src that won't change, 
# and then another layer that compiles our source.
RUN echo 'fn main() {}' >> /tmp/oryx/dummy.rs

RUN sed -i 's|src/main.rs|dummy.rs|g' Cargo.toml
RUN env CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --target x86_64-unknown-linux-musl --release

RUN sed -i 's|dummy.rs|src/main.rs|g' Cargo.toml
COPY . /tmp/oryx
RUN env CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --target x86_64-unknown-linux-musl --release


FROM docker.io/alpine:latest

RUN apk add --no-cache tini

COPY --from=cargo-build /tmp/oryx/target/x86_64-unknown-linux-musl/release/oryx /
WORKDIR /

ENV RUST_LOG=INFO
CMD ["./oryx"]
