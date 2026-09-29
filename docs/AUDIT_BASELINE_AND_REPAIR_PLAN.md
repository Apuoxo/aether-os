# Aether OS — Engineering Audit Baseline & Repair Plan

Baseline: 2026-09-23  
Current-source refresh: 2026-09-29  

This document preserves architectural findings without allowing superseded findings to masquerade as current defects. Current truth is established by source, tests and docs/STATUS.md.

## Corrected at source level

- Syscall frame/ABI mismatch: corrected in isr.s / handlers.rs; deterministic runtime closure is still pending.
- 64-bit TSS descriptor encoding: corrected in gdt.rs; deterministic CPL3 runtime closure is still pending.
- Process -> Personality ownership: implemented and corrected for the legacy constructor.
- Capability handle generation: hardened against revoke/reuse ambiguity.
- ELF page ownership bound: expanded and checked; loader arithmetic/file-size validation was added.

## Still open

- deterministic Ring3 syscall/TSS runtime evidence;
- complete object-authority capability model;
- actual timer-driven/preemptive scheduler and thread model;
- strict separation of RAM AetherFS from any persistent disk-write path;
- unified on-disk AetherFS/VFS model;
- AHCI DMA/addressability limitations and broader storage regressions;
- Multiboot memory-map-driven PMM;
- complete current-address-space validation for userspace memory;
- any remaining fixed display/mouse geometry assumptions;
- Intel Gen6 display completion without unsafe GGTT/GSM access;
- real loadable Personality modules and compatibility layers.

## Important documentation corrections

1. Aether is currently a native monolithic kernel with integrated desktop/storage/driver code, not yet the final microvisor-style modular architecture.
2. Capabilities are real infrastructure, not yet universal object authority.
3. Personality ownership is real; dynamically unloadable personality binaries are not.
4. The scheduler remains cooperative/basic.
5. QEMU Ring3 evidence is not equivalent to real-AH532 hardware evidence.
6. The desktop GUI terminal is dispatched by kernel/src/desktop.rs; shell.rs must not be assumed to own those commands.
7. HDA controller detection is not equivalent to audible PCM playback.
8. Intel Wi-Fi firmware/ALIVE/TX transport bring-up is not equivalent to working scan/networking.

## Repair discipline

For each hardware or kernel repair:
1. inspect current source;
2. make one controlled change;
3. build/CI;
4. run deterministic QEMU tests where applicable;
5. test AH532 when hardware-dependent;
6. update docs/STATUS.md and the relevant subsystem log;
7. do not close a finding without matching source invariant and evidence.

## Historical detailed findings

The original detailed findings remain in docs/AUDIT_2026-09-23.md. They are snapshots of the repository at that time, not a live defect tracker.
