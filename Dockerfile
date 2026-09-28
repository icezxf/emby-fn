FROM rust:1.85-alpine AS builder
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig
WORKDIR /app
COPY Cargo.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && rm -rf src
COPY src ./src
RUN touch src/main.rs && cargo build --release

FROM alpine:3.19
RUN apk --no-cache add ca-certificates tzdata && adduser -D -u 1000 app
ENV TZ=Asia/Shanghai
USER app
WORKDIR /app
COPY --from=builder /app/target/release/fnos-emby-gateway .
EXPOSE 8007
CMD ["./fnos-emby-gateway"]
