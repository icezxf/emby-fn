FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app

COPY Cargo.toml ./
COPY src ./src

RUN cargo build --release

FROM alpine:3.19

RUN apk --no-cache add ca-certificates tzdata && adduser -D -u 1000 app
ENV TZ=Asia/Shanghai

USER app
WORKDIR /app
COPY --from=builder /app/target/release/fnos-emby-gateway .

EXPOSE 8007
CMD ["./fnos-emby-gateway"]