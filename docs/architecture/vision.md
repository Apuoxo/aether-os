# Vision — Aether

## Architectural vision

Aether's long-term idea is a native operating system whose kernel is not permanently tied to one application ecosystem.

The target is a polymorphic native kernel in which Linux, Android and Windows can be represented as unloadable Personalities rather than as hosted virtual machines or CPU emulators.

This is a design goal, not the current implementation.

## Principles

- Native first: programs execute natively on the CPU; Aether does not emulate CPU instructions.
- No host OS dependency: Aether is intended to boot and run as the operating system itself.
- Capability-oriented authority: kernel resources are intended to be mediated by capabilities.
- Personality isolation: a process belongs to one Personality for its lifetime.
- Lazy loading: Personality implementation should be loaded only when needed.
- Unloadability: Personality-owned code/data should be reclaimable when no process depends on it.

## Current implementation boundary

Today the repository contains:
- a native x86_64 Rust kernel;
- Ring3/process foundations;
- capability tables and checks for implemented paths;
- native desktop and applications;
- hardware-driver bring-up for storage, graphics, HDA and Intel Wi-Fi;
- Personality IDs, manager state and process ownership.

Today it does not contain complete Linux, Android or Windows compatibility personalities or a real external Personality binary/module loader.

For the live state, use docs/STATUS.md and the source tree.
