# Estágio de Build
FROM rustlang/rust:nightly-slim AS builder
WORKDIR /app

# Instala apenas dependências de sistema essenciais
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libsqlite3-dev build-essential git \
    && rm -rf /var/lib/apt/lists/*

COPY . .

# Variável de ambiente para o KoShelf não tentar rodar o NPM
ENV KOSHELF_SKIP_NPM_INSTALL=1

# Compila o binário Rust com as suas correções no web.rs
RUN cargo build --release

# Estágio Final
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y \
    libssl3 libsqlite3-0 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/koshelf /usr/local/bin/koshelf

# Copia a pasta dist existente no seu repositório
# Certifique-se que ela está no seu GitHub. Se não estiver, use 'mkdir dist' para não dar erro
RUN mkdir -p dist
COPY --from=builder /app/dist ./dist 

EXPOSE 3009
ENTRYPOINT ["koshelf"]
