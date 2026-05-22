#!/bin/bash
# setup_ubuntu.sh — AOSP build host preparation for Ubuntu 22.04 LTS
# WHY: Idempotent script to install all necessary prerequisites for building AOSP.
# Based on official AOSP documentation: https://source.android.com/docs/setup/start/initializing

set -euo pipefail

echo "==> Starting YBOS AOSP Build Host Setup (Ubuntu 22.04 LTS)"

# Ensure we are running on Ubuntu 22.04
if ! grep -q "22.04" /etc/os-release; then
    echo "WARNING: This script is designed for Ubuntu 22.04 LTS. Your version may differ."
fi

# Update package list
echo "==> Updating package lists..."
sudo apt-get update

# Install required packages for AOSP build
echo "==> Installing AOSP build prerequisites..."
sudo apt-get install -y \
    git-core \
    gnupg \
    flex \
    bison \
    build-essential \
    zip \
    curl \
    zlib1g-dev \
    libc6-dev-i386 \
    libncurses5 \
    x11proto-core-dev \
    libx11-dev \
    lib32z1-dev \
    libgl1-mesa-dev \
    libxml2-utils \
    xsltproc \
    unzip \
    fontconfig \
    python3 \
    python-is-python3 \
    libssl-dev \
    bc \
    rsync \
    ccache \
    git-lfs \
    shellcheck

# Install OpenJDK 11 and 17 (AOSP 14/15 requirements)
echo "==> Installing OpenJDK 11 and 17..."
sudo apt-get install -y openjdk-11-jdk openjdk-17-jdk

# Install 'repo' tool if not present
if ! command -v repo >/dev/null 2>&1; then
    echo "==> Installing 'repo' tool..."
    mkdir -p "$HOME/bin"
    curl https://storage.googleapis.com/git-repo-downloads/repo > "$HOME/bin/repo"
    chmod a+x "$HOME/bin/repo"
    echo "NOTE: Make sure $HOME/bin is in your PATH."
else
    echo "==> 'repo' tool already installed."
fi

# Configure git-lfs
echo "==> Configuring git-lfs..."
git lfs install

# Setup ccache
if [ ! -d "$HOME/.ccache" ]; then
    echo "==> Initializing ccache..."
    mkdir -p "$HOME/.ccache"
fi

echo "==> Setup complete!"
echo "Please restart your shell or run 'source ~/.bashrc' if you added $HOME/bin to your PATH."
echo "Code implemented with help from AI Agents Claude, Codex, Jules."
