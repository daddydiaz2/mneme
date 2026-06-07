# ============================================================
# Stage 1: Build stage
# ============================================================
FROM rust:slim-bookworm AS builder

# Instalar dependencias necesarias para compilar (sqlite3, clang, pkg-config, ssl)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    clang \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/mneme

# Copiar Cargo para pre-compilar y cachear dependencias
COPY Cargo.toml Cargo.lock ./

# Crear stubs mínimos para compilar dependencias en cache
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs && \
    cargo build --release --features embeddings,plugins

# Eliminar stubs
RUN rm -rf src/

# Copiar código fuente real y migraciones
COPY src ./src
COPY migrations ./migrations

# Tocar archivos para asegurar que cargo reconstruya el código real
RUN touch src/main.rs src/lib.rs && \
    cargo build --release --features embeddings,plugins

# ============================================================
# Stage 2: Final runtime stage
# ============================================================
FROM debian:bookworm-slim AS final

# Instalar dependencias de runtime (openssl, sqlite y certificados ca para HTTPS sync)
RUN apt-get update && apt-get install -y \
    libssl3 \
    libsqlite3-0 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Crear directorios para datos, plugins y logs
RUN mkdir -p /app/data /app/plugins

WORKDIR /app

# Copiar binario compilado desde la etapa anterior
COPY --from=builder /usr/src/mneme/target/release/mneme /usr/local/bin/mneme

# Configurar variables de entorno por defecto
ENV MNEME_DATABASE_PATH=/app/data/mneme.db
ENV MNEME_HOST=0.0.0.0
ENV MNEME_PORT=8080
ENV MNEME_PLUGINS_DIR=/app/plugins

# Exponer el puerto HTTP API
EXPOSE 8080

# Salud del contenedor usando el endpoint health
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/v1/health || exit 1

# Crear un usuario no privilegido y darle permisos sobre /app
RUN groupadd -g 10001 mneme && \
    useradd -u 10001 -g mneme -s /bin/bash -m mneme && \
    chown -R mneme:mneme /app

USER mneme

# Punto de entrada por defecto: iniciar el servidor HTTP
ENTRYPOINT ["mneme", "serve"]
