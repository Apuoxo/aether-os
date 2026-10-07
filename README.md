# Aether OS

Aether OS starts again from a clean foundation.

The project has two inseparable goals:

1. **A complete Linux operating system** — not an embedded image, not a reduced kernel, and not a hardware-specific experiment. Aether must preserve the Linux ABI/API, hardware support, drivers, filesystems, networking, graphics, audio, security, virtualization, services, desktop, applications, and normal Linux workflows.
2. **Absolute AI integration** — AI is a first-class system intelligence with a structured way to observe, understand, plan, and control the entire operating system.

## Architecture

```
                         AETHER AI
                             |
                    AETHER SYSTEM INTERFACE
                             |
                        AETHER CORE
                             |
              +--------------+--------------+
              |                             |
        Linux userspace              Aether services
              |                             |
              +--------------+--------------+
                             |
                    FULL LINUX KERNEL
                             |
                         HARDWARE
```

### Full Linux

Aether does not replace Linux with a custom kernel abstraction.

Linux remains the real operating-system foundation:

- full x86_64 Linux kernel;
- standard Linux ABI/API;
- broad hardware and driver support;
- CPU/SMP, memory management and power management;
- storage and filesystems;
- networking, Wi-Fi and Bluetooth;
- GPU/DRM/KMS and display;
- audio and USB;
- security, namespaces, cgroups and containers;
- virtualization;
- standard services and desktop environments;
- normal Linux applications and development tools.

AI must never be a prerequisite for Linux to boot or operate.

### Absolute AI integration

Aether AI must not be limited to a chat window or a root shell.

The system will provide a structured interface for:

- system state and hardware observation;
- processes and resources;
- files and storage;
- networking;
- devices and drivers;
- services;
- applications;
- desktop state and interaction;
- system events;
- diagnostics;
- controlled configuration and administration;
- build, test, deployment and recovery workflows.

The AI layer will use Linux capabilities rather than replacing Linux interfaces. Existing Linux applications must continue to work normally whether AI is enabled or disabled.

## Compatibility principle

> **Aether adds capabilities to Linux; it does not remove Linux capabilities.**

A normal Linux program should not need to know that it is running on Aether.

The AI integration must be additive, observable, permission-aware and recoverable.

## Development principles

- Correctness before speed.
- No artificial functional limits in the Linux foundation.
- One verified architectural step at a time.
- Every functional change must be buildable and testable.
- AI must remain isolated from kernel-critical operation.
- Dangerous AI actions require explicit system capabilities and policy.
- Failed system generations must be recoverable.
- Hardware-specific work belongs in the Linux driver ecosystem whenever possible, not in custom bare-metal replacements.

## Starting point

This repository is intentionally reset to an empty project foundation.

Nothing from the previous bare-metal Rust implementation is part of the new architecture.

The first implementation stages are:

1. establish a reproducible full-Linux build;
2. boot a real Linux system in QEMU;
3. establish the complete userspace/root filesystem;
4. establish Aether Core as the system integration boundary;
5. define the Aether System Interface and system-state model;
6. add the AI model bridge;
7. integrate AI observation and control incrementally;
8. validate the same architecture on real hardware.

The project will grow from this foundation without sacrificing Linux compatibility or AI integration.
