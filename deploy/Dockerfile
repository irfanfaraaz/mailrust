# Build from the repo root: docker build -f deploy/Dockerfile .
FROM rust:1.99-bookworm AS build
WORKDIR /app
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/mailrust /usr/local/bin/mailrust
USER 10001
ENTRYPOINT ["mailrust"]
CMD ["serve"]
