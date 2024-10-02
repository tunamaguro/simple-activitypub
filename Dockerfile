FROM rust:1.80.0-slim-bookworm as dev

RUN apt-get update -y && \
    apt-get install -y \
    sudo \
    git \
    # openssl
    # https://docs.rs/openssl/latest/openssl/#
    libssl-dev \
    pkg-config

ARG USERNAME=vscode
ARG GROUPNAME=vscode
ARG UID=1000
ARG GID=1000
ARG PASSWORD=vscode
RUN groupadd -g $GID $GROUPNAME && \
    useradd -m -s /bin/bash -u $UID -g $GID -G sudo $USERNAME && \
    echo $USERNAME:$PASSWORD | chpasswd && \
    echo "$USERNAME   ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Add completions
RUN echo "source /usr/share/bash-completion/completions/git" >> /home/vscode/.bashrc
RUN echo "source <( rustup completions bash )" >> /home/vscode/.bashrc
RUN echo "source <( rustup completions bash cargo )" >> /home/vscode/.bashrc

USER ${USERNAME}

RUN rustup component add rustfmt clippy
RUN cargo install cargo-watch