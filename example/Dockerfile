# Conformance example app (axum + askama). Build with the package folder as
# the context: docker build -f example/Dockerfile .
FROM rust:1-slim AS build
WORKDIR /src
# The crate from this repo's own source (a path dependency).
COPY Cargo.toml README.md ./
COPY src/ src/
COPY example/ example/
WORKDIR /src/example
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=build /src/example/target/release/privatus-analytics-example /example
EXPOSE 8080
USER 65534
ENTRYPOINT ["/example"]
