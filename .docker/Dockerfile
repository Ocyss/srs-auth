FROM rust:1 AS chef

RUN apt-get update && apt-get install -y \
    musl-tools \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-musl

ENV CC_x86_64_unknown_linux_musl=gcc

# RUN cargo install cargo-chef

RUN curl -L --proto '=https' --tlsv1.2 -sSf \
    https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh \
    | bash

RUN cargo-binstall cargo-chef

# RUN cargo-binstall dioxus-cli@0.8.0-alpha.1

RUN curl -sSL https://dioxus.dev/install.sh \
    | bash -s -- v0.8.0-alpha.1

RUN curl -fsSL https://bun.com/install | bash

WORKDIR /app

FROM chef AS planner

RUN mkdir -p /temp/node
COPY package.json bun.lock /temp/node/
RUN cd /temp/node && /root/.bun/bin/bun install --frozen-lockfile

COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS builder

COPY --from=planner /temp/node/node_modules node_modules
COPY --from=planner /app/recipe.json recipe.json
COPY --from=planner /app/vendor ./vendor

RUN cargo chef cook \
    --release \
    --target x86_64-unknown-linux-musl \
    --recipe-path recipe.json

COPY . .

RUN /root/.bun/bin/bun x tailwindcss \
    -i tailwind.css \
    -o ./assets/tailwind.css

RUN /root/.dx/bin/dx bundle \
    --platform web \
    --release \
    --target x86_64-unknown-linux-musl

FROM alpine AS runtime

COPY --from=builder /app/target/dx/srs-auth/release/web/ /app

ENV PORT=8080
ENV IP=0.0.0.0
EXPOSE 8080

WORKDIR /app
ENTRYPOINT [ "/app/server" ]