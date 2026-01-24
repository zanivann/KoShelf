# Estágio de Build
FROM rustlang/rust:nightly-slim AS builder
WORKDIR /app

# Instala dependências de sistema e Node.js 20 (necessário para o CSS/JS)
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libsqlite3-dev build-essential git curl \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

COPY . .

# 1. Instalamos as dependências do Node primeiro
RUN npm install

# 2. Agora o cargo build vai conseguir rodar o build.rs 
# que gera automaticamente o compiled_style.css e os .js
RUN cargo build --release

# Estágio Final
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y \
    libssl3 libsqlite3-0 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/koshelf /usr/local/bin/koshelf
# O KoShelf precisa da pasta dist para servir imagens/icones
COPY --from=builder /app/dist ./dist 

EXPOSE 3009
ENTRYPOINT ["koshelf"]
