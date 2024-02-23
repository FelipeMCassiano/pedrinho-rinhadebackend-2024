# Use a imagem oficial do Rust como imagem pai
FROM rust:1.76-buster as builder

# Defina o diretório de trabalho no contêiner como /app
WORKDIR /app

# Defina a variável de ambiente SQLX_OFFLINE como true

# Copie o conteúdo do diretório atual para o contêiner em /app
COPY . .

# Construa o aplicativo no modo de lançamento
RUN cargo build --release

# Use a imagem mínima do Debian como imagem pai
FROM debian:buster-slim

# Defina o diretório de trabalho no contêiner como /usr/local/bin
WORKDIR /usr/local/bin

# Copie o aplicativo compilado do contêiner builder para o contêiner atual
COPY --from=builder /app/target/release/rinha-de-backend-2024-q1 .

# Atualize os pacotes e instale o openssl
RUN apt-get update && apt install -y openssl

# Execute o aplicativo quando o contêiner for iniciado
CMD ["./rinha-de-backend-2024-q1"]


