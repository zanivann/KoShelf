# Estágio de Build usando Rust Nightly
FROM rustlang/rust:nightly-slim AS builder
WORKDIR /app

# Instalando dependências essenciais para SQLite e compilação
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    build-essential \
    git \
    && rm -rf /var/lib/apt/lists/*

COPY . .

# Habilita funcionalidades instáveis do Cargo se necessário e compila
RUN cargo build --release

# Estágio Final para o Raspberry Pi 5
FROM debian:bookworm-slim
WORKDIR /app

# Dependências de execução (runtime)
RUN apt-get update && apt-get install -y \
    libssl3 \
    libsqlite3-0 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copia o binário e a pasta do frontend
COPY --from=builder /app/target/release/koshelf /usr/local/bin/koshelf
COPY --from=builder /app/dist ./dist 

EXPOSE 3009
ENTRYPOINT ["koshelf"]
