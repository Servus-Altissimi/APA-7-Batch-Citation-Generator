FROM rust:1-trixie AS build
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown
RUN cargo install cargo-binstall --locked \
 && cargo binstall -y dioxus-cli@0.7.3 \
 || cargo install dioxus-cli --version 0.7.3 --locked
WORKDIR /app
COPY . .
RUN dx build --release --platform web
ARG HEAD_SNIPPET_B64=
RUN if [ -n "$HEAD_SNIPPET_B64" ]; then \
        echo "$HEAD_SNIPPET_B64" | base64 -d > /tmp/head.html; \
        awk -v ins="$(cat /tmp/head.html)" '/<\/head>/{print ins} {print}' \
            target/dx/apaciter/release/web/public/index.html > /tmp/index.html; \
        mv /tmp/index.html target/dx/apaciter/release/web/public/index.html; \
    fi

FROM nginx:alpine
COPY --from=build /app/target/dx/apaciter/release/web/public /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 8080
