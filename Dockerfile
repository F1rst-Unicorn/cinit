# cinit: process initialisation program for containers
# Copyright (C) 2019 The cinit developers
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

FROM rust:buster

RUN rustup target add x86_64-unknown-linux-musl
RUN rustup component add clippy
RUN rustup component add rustfmt
RUN cargo install cargo-audit

# Development dependencies
RUN apt update && \
        apt install -y --no-install-recommends \
                musl \
                musl-tools \
                musl-dev \
                systemd \
                python3-systemd \
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
