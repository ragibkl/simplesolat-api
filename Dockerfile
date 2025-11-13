## builder
FROM alpine:3.22 AS builder

WORKDIR /code/monosolat-api

# install system dependencies
RUN apk add build-base \
    cargo \
    clang \
    clang-dev \
    clang-libs \
    linux-headers \
    libpq \
    libpq-dev \
    openssl \
    openssl-dev \
    rust

# setup build dependencies
RUN cargo init .
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release
RUN rm -rf ./src/

# copy code files
COPY /src/ ./src/

# build code
RUN touch ./src/main.rs
RUN touch ./src/bin/sync.rs
RUN cargo build --release


## runtime
FROM alpine:3.22 AS prod

WORKDIR /app

# install runtime dependencies
RUN apk add openssl libgcc libstdc++ libpq

# set default logging, can be overridden
ENV RUST_LOG=info

# copy data
COPY data /app/data

# copy binary
COPY --from=builder /code/monosolat-api/target/release/monosolat-api /usr/local/bin/monosolat-api
COPY --from=builder /code/monosolat-api/target/release/sync /usr/local/bin/sync

# set entrypoint
ENTRYPOINT ["/usr/local/bin/monosolat-api"]
