# Estágio de Build
FROM rust:1.75-slim as builder
WORKDIR /app

# Instalando dependências de sistema fundamentais
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    build-essential \
    git \
    && rm -rf /var/lib/apt/lists/*

COPY . .

# Compilação otimizada
RUN cargo build --release

# Estágio Final
FROM debian:bookworm-slim
WORKDIR /app

# Instalando dependências de execução
RUN apt-get update && apt-get install -y \
    libssl3 \
    libsqlite3-0 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/koshelf /usr/local/bin/koshelf
# Copia o frontend compilado
COPY --from=builder /app/dist ./dist 

EXPOSE 3009
ENTRYPOINT ["koshelf"]
