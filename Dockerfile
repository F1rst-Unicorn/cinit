FROM rust:stretch

RUN rustup target add x86_64-unknown-linux-musl
RUN rustup component add clippy

# Development dependencies
RUN apt update && \
        apt install -y --no-install-recommends \
                musl \
                musl-tools \
                musl-dev \
                python3-yaml && \
        rm -rf /var/lib/apt/lists/*

COPY scripts/container/compile-libcap /tmp
RUN /tmp/compile-libcap

ARG USER_ID
ARG GROUP_ID

RUN groupadd --gid $GROUP_ID builder || true
RUN groupadd --gid 1409 testgroup || true

RUN useradd -M -N --uid $USER_ID --gid $GROUP_ID builder || true
RUN useradd -d /home/testuser -m -N --uid 1409 --gid testgroup testuser

RUN echo 'ENV_PATH PATH=/usr/local/cargo/bin:/usr/local/bin:/usr/bin:/bin:/usr/local/games:/usr/games' \
        >> /etc/login.defs

RUN ln -sf /usr/share/zoneinfo/Europe/Berlin /etc/localtime

CMD ["exit"]
