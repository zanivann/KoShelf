# Estágio de Build
FROM rustlang/rust:nightly-slim AS builder
WORKDIR /app

# Instala dependências e Node.js
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libsqlite3-dev build-essential git curl \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

COPY . .

# Compila o binário e o frontend
RUN cargo build --release

# Verificação: Lista os arquivos para garantir que sabemos onde o frontend foi parar
RUN ls -R | grep dist || echo "Pasta dist não encontrada na raiz"

# Estágio Final
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y \
    libssl3 libsqlite3-0 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/koshelf /usr/local/bin/koshelf

# Se o erro persistir, tente comentar a linha abaixo para testar apenas o backend
COPY --from=builder /app/dist ./dist 

EXPOSE 3009
ENTRYPOINT ["koshelf"]
