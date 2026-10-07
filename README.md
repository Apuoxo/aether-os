# Aether OS — Linux base

Aether OS is restarting on a Linux kernel foundation.

## Direction

- Kernel: upstream Linux
- Target hardware: Fujitsu LIFEBOOK AH532 (x86_64, Sandy Bridge)
- Userland, desktop, networking, drivers and Aether-specific integration will be built above Linux.
- Previous Rust bare-metal kernel work is intentionally removed from the active tree.

The Linux source is pinned as a Git submodule at `linux/`.

Upstream kernel commit pinned for this reset:
`602042bf29f6efde39cfb5fdd9289bf4854bc0c5`

This repository will contain Aether-specific configuration, integration and tooling; Linux kernel source remains upstream-owned.
