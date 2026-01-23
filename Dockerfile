# Estágio de Build: Compila o binário
FROM rust:1.75-slim as builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev build-essential git
COPY . .
RUN cargo build --release

# Estágio Final: Imagem leve para rodar o app
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/koshelf /usr/local/bin/koshelf
# O KoShelf pode precisar da pasta dist se houver frontend
COPY --from=builder /app/dist ./dist 

EXPOSE 3009
ENTRYPOINT ["koshelf"]