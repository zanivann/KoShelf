# Estágio de Build
FROM rustlang/rust:nightly-slim AS builder
WORKDIR /app

# Instala dependências de sistema, Node.js e npm
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libsqlite3-dev build-essential git curl \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

COPY . .

# PASSO CRUCIAL: Instala dependências do Node e gera a pasta dist manualmente
# Se o comando de build do seu frontend for diferente de 'npm run build', ajuste abaixo
RUN npm install && npm run build

# Compila o binário Rust (agora com a pasta dist já garantida)
RUN cargo build --release

# Estágio Final
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y \
    libssl3 libsqlite3-0 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copia o binário compilado
COPY --from=builder /app/target/release/koshelf /usr/local/bin/koshelf

# Copia a pasta dist que acabamos de gerar manualmente
COPY --from=builder /app/dist ./dist 

EXPOSE 3009
ENTRYPOINT ["koshelf"]
