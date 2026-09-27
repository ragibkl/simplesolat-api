## builder
FROM alpine:3.22 AS builder

WORKDIR /code/simplesolat-api

# install system dependencies
RUN apk add build-base \
    cargo \
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
RUN cargo build --release


## runtime
FROM alpine:3.22 AS prod

WORKDIR /app

# install runtime dependencies
RUN apk add ca-certificates openssl libgcc libstdc++

# set default logging, can be overridden
ENV RUST_LOG=info

# copy binary
COPY --from=builder /code/simplesolat-api/target/release/simplesolat-api /usr/local/bin/simplesolat-api

# set entrypoint
ENTRYPOINT ["/usr/local/bin/simplesolat-api"]
