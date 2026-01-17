# --- Stage 1: Build (Unchanged) ---
FROM ubuntu:22.04 AS builder
RUN apt-get update && apt-get install -y \
    build-essential libncursesw5-dev libsqlite3-dev liblua5.1-0-dev \
    zlib1g-dev libsdl2-dev libsdl2-image-dev libsdl2-mixer-dev \
    libsdl2-ttf-dev libpng-dev pkg-config python3 python3-pip \
    python3-yaml git bison flex && rm -rf /var/lib/apt/lists/*
WORKDIR /build
RUN git clone --recursive https://github.com/crawl/crawl.git
WORKDIR /build/crawl/crawl-ref/source
RUN git checkout 0.33.1
RUN make WEBTILES=y -j$(nproc)

# --- Stage 2: Runtime environment ---
FROM ubuntu:22.04

# Only install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 python3-tornado python3-yaml \
    libncursesw5 libsqlite3-0 liblua5.1-0 zlib1g libpng16-16 \
    sqlite3 bash \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 1. Copy required assets from builder
COPY --from=builder /build/crawl/crawl-ref/source/ /app/
COPY --from=builder /build/crawl/crawl-ref/settings/ /app/settings/

# 2. Create the config.py using a Heredoc for better readability
RUN cat <<EOF > /app/webserver/config.py
bind_address = "0.0.0.0"
bind_port = 8080
bind_nonsecure = True
secret_key = "something-very-secret-here"
server_id = "local-container-server"
allow_registration = True
crawl_binary = "/app/crawl"
rc_path = "/app/saves/rcs"
morgue_path = "/app/saves/morgues"
init_player_program = "/app/util/webtiles-init-player.sh"
data_path = "/app/dat/"
static_path = "/app/webserver/static"
template_path = "/app/webserver/templates"
client_path = "/app/webserver/game_data/"
game_data_path = "/app/webserver/game_data/"
password_db = "/app/saves/webtiles.db"
dgl_status_file = "/app/saves/dgl-status.txt"
games = {}
ssl_options = None
umask = 0o022
EOF

# 3. Final Directory/Permission Prep
RUN mkdir -p /app/saves/rcs /app/saves/morgues /app/settings && \
    echo "# Default DCSS Settings" > /app/settings/init.txt && \
    # Patch the script to use absolute paths
    sed -i 's|\.\./settings/init\.txt|/app/settings/init.txt|g' /app/util/webtiles-init-player.sh && \
    # Broad permissions for rootless Podman compatibility
    chmod -R 777 /app/saves /app/settings /app/util && \
    chmod +x /app/util/*.sh

ENV PYTHONPATH=/app/webserver
EXPOSE 8080

# Use the absolute path for the entry point
CMD ["python3", "/app/webserver/server.py"]
