# Aether OS — AI-first system

Aether is an AI-first operating environment. It is not intended to become another Linux distribution with an AI application attached.

## Architecture direction

- **Aether AI** is the central system entity.
- **Aether Core** is the privileged system layer and IPC boundary through which the AI can observe and operate the machine.
- **Linux** is the initial low-level foundation: scheduler, memory management, drivers, PCI, networking, graphics, audio, filesystems and other hardware mechanisms.
- **Linux application compatibility is mandatory:** the long-term Aether kernel must continue to run ordinary Linux software and support the Linux driver ecosystem while Aether replaces or extends lower layers.
- **Self-development is a first-class goal:** Aether AI must ultimately be able to inspect, modify, build, test and deploy changes to Aether Core and the kernel itself.

The current tree is intentionally small. We are building the system from a verified Linux boot foundation upward, one architectural layer at a time.

## Current milestone

The first Aether Core bootstrap provides a real privileged process and Unix-domain IPC socket inside the boot environment. It is infrastructure, not yet the AI itself.

Linux source remains an upstream Git submodule at linux/.
