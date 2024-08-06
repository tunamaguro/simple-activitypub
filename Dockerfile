FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm as dev

# Add completions
RUN echo "source /usr/share/bash-completion/completions/git" >> /home/vscode/.bashrc
RUN echo "source <( rustup completions bash )" >> /home/vscode/.bashrc
RUN echo "source <( rustup completions bash cargo )" >> /home/vscode/.bashrc