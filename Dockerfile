# Estágio de Build
FROM rustlang/rust:nightly-slim AS builder
WORKDIR /app

# Instala dependências de sistema e Node.js
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libsqlite3-dev build-essential git curl \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

COPY . .

# 1. Instalamos as dependências do Node para que o build.rs funcione
RUN npm install

# 2. O cargo build compila o binário e embuti o CSS/JS automaticamente
RUN cargo build --release

# Estágio Final
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y \
    libssl3 libsqlite3-0 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copiamos apenas o binário, que já contém o frontend embutido
COPY --from=builder /app/target/release/koshelf /usr/local/bin/koshelf

# Criamos uma pasta dist vazia apenas para evitar erros de inicialização do servidor
RUN mkdir -p dist

EXPOSE 3009
ENTRYPOINT ["koshelf"]
