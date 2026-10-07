# Aether OS architecture

## Base

Aether is no longer a standalone Rust/no_std kernel project.

The base operating-system kernel is upstream Linux. Aether-specific work is layered above it:

1. Linux kernel and hardware support
2. AH532-specific kernel configuration and validation
3. Aether system services
4. Desktop/window system
5. Application/runtime integration
6. AI/model bridge

## Rule

Prefer existing Linux kernel facilities over reimplementing kernel subsystems.

The first engineering milestone is a reproducible Linux boot on the AH532. Only after that will Aether-specific desktop and application work resume.
