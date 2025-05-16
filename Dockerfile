####################################################################################################
## Builder
####################################################################################################
FROM rust:latest AS builder

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev make ca-certificates 
#git
RUN update-ca-certificates

# Create appuser
ENV USER=app
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"


WORKDIR /app

COPY bf-common-utils/src bf-common-utils/src
COPY bf-common-utils/Cargo* bf-common-utils/.
COPY auth2fa/src auth2fa/src/
COPY auth2fa/Cargo* auth2fa/.

ENV CARGO_BIN_NAME=out

WORKDIR /app/auth2fa
RUN cargo build --target x86_64-unknown-linux-musl --release --bin ${CARGO_BIN_NAME}


####################################################################################################
## Final image
####################################################################################################
FROM alpine:latest

# Add CA certificates
RUN apk update && apk add --no-cache ca-certificates

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /app

COPY --from=builder /app/auth2fa/target/x86_64-unknown-linux-musl/release/out ./

USER app:app

CMD ["/app/out"]
