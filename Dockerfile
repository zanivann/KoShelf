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

# Criamos as variáveis para enganar a trava do build.rs
ENV KOSHELF_SKIP_NPM_INSTALL=1
ENV KOSHELF_SKIP_NODE_BUILD=1

# Instalamos as dependências apenas para a pasta node_modules existir
# O '--no-scripts' evita que ele tente rodar builds automáticos que falham
RUN npm install --no-scripts

# Compila o binário Rust (agora o build.rs verá que node_modules existe)
RUN cargo build --release

# Estágio Final
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y \
    libssl3 libsqlite3-0 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/koshelf /usr/local/bin/koshelf

# Garante que a pasta dist exista (mesmo que vazia ou vinda do seu repo)
RUN mkdir -p dist
COPY --from=builder /app/dist ./dist 

EXPOSE 3009
ENTRYPOINT ["koshelf"]
