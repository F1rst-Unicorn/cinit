FROM rust:stretch

RUN rustup target add x86_64-unknown-linux-musl

ARG USER_ID
ARG GROUP_ID

RUN groupadd --gid $GROUP_ID builder || true

RUN useradd -M -N --uid $USER_ID --gid $GROUP_ID builder || true

RUN echo 'ENV_PATH        PATH=/usr/local/cargo/bin:/usr/local/bin:/usr/bin:/bin:/usr/local/games:/usr/games' >> /etc/login.defs

CMD ["exit"]
