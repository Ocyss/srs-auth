FROM rust:1 AS chef

RUN cargo install cargo-chef

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/DioxusLabs/dioxus/refs/heads/main/.github/install.sh | bash -s -- v0.8.0-alpha.1
RUN curl -fsSL https://bun.com/install | bash

WORKDIR /app

FROM chef AS planner

RUN mkdir -p /temp/node
COPY package.json bun.lock /temp/node/
RUN cd /temp/node && bun install --frozen-lockfile

COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS builder

COPY --from=planner /temp/node/node_modules node_modules
COPY --from=planner /app/recipe.json recipe.json
COPY --from=planner /app/vendor ./vendor

RUN cargo chef cook --release --recipe-path recipe.json

COPY . .

RUN bun x tailwindcss -i tailwind.css -o ./assets/tailwind.css
RUN /root/.dx/bin/dx bundle --platform web

FROM chef AS runtime
COPY --from=builder /app/target/dx/srs-auth/release/web/ /usr/local/app

ENV PORT=8080
ENV IP=0.0.0.0
EXPOSE 8080

WORKDIR /usr/local/app
ENTRYPOINT [ "/usr/local/app/server" ]