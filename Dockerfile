## builder
FROM alpine:3.22 AS builder

WORKDIR /code/simplesolat-api

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
COPY /migrations/ ./migrations/
COPY /src/ ./src/

# build code
RUN touch ./src/main.rs
RUN touch ./src/bin/sync.rs
RUN cargo build --release
RUN ls target/*


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
COPY --from=builder /code/simplesolat-api/target/release/simplesolat-api /usr/local/bin/simplesolat-api
COPY --from=builder /code/simplesolat-api/target/release/sync /usr/local/bin/sync

# set entrypoint
ENTRYPOINT ["/usr/local/bin/simplesolat-api"]
