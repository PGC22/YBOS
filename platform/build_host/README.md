# AOSP Build Host Preparation

This directory contains the necessary tools and documentation to prepare an Ubuntu 22.04 LTS host for building YBOS (AOSP-based).

## Hardware / Cloud Recommendations

Building AOSP is a resource-intensive process. We recommend the following minimum specifications:

- **CPU**: Octa-core (or more)
- **RAM**: 32GB minimum (64GB recommended)
- **Disk**: 250GB minimum (SSD/NVMe highly recommended)
- **OS**: Ubuntu 22.04 LTS (Jammy Jellyfish)

### Cloud VM Recommendations

If you don't have a local machine meeting these specs, we recommend:
- **Hetzner**: CCX33 or larger
- **AWS**: c6i.4xlarge or larger
- **Azure**: Standard_F16s_v2 or larger

## Prerequisites Setup

We provide an idempotent script to install all necessary packages and tools.

```bash
chmod +x setup_ubuntu.sh
./setup_ubuntu.sh
```

The script installs:
- Build essentials (git, curl, zip, etc.)
- OpenJDK 11 and 17
- Python 3
- `repo` tool
- `ccache` for faster incremental builds
- `git-lfs`

## Manual Steps

After running the script, ensure `$HOME/bin` is in your `PATH`:

```bash
export PATH=$HOME/bin:$PATH
```

You should also configure your Git identity if you haven't already:

```bash
git config --global user.email "you@example.com"
git config --global user.name "Your Name"
```

Code implemented with help from AI Agents Claude, Codex, Jules.
