# --- Build image
FROM rust:1.97.1-alpine3.24@sha256:3c38f3f82c2f3d73da3b38e18d279393a04cb43ddded0e35088a8c3324d40900 AS builder

WORKDIR /usr/src/beaver
COPY . .
RUN apk add --no-cache \
    musl-dev \
    build-base \
    cmake \
    perl \
    make \
    git

RUN cargo build --release

# --- Final image
FROM scratch
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

COPY --from=builder /usr/src/beaver/target/release/cli /cli
COPY --from=builder /usr/src/beaver/target/release/webhook /webhook

COPY examples/tools /tools

WORKDIR /tmp
WORKDIR /

EXPOSE 8080/tcp
ENTRYPOINT ["/webhook"]
