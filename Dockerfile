# Estágio de Build
FROM rust:1.84-slim AS builder
WORKDIR /app

# Instalando dependências de sistema para SQLite e SSL
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    build-essential \
    git \
    && rm -rf /var/lib/apt/lists/*

COPY . .

# Compilação em modo Release
RUN cargo build --release

# Estágio Final
FROM debian:bookworm-slim
WORKDIR /app

# Dependências de runtime
RUN apt-get update && apt-get install -y \
    libssl3 \
    libsqlite3-0 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/koshelf /usr/local/bin/koshelf
# O KoShelf geralmente precisa da pasta dist para o frontend
COPY --from=builder /app/dist ./dist 

EXPOSE 3009
ENTRYPOINT ["koshelf"]
