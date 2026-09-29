# Aether Architecture Overview

## 1. What Aether is today

Aether is currently a native x86_64 Rust operating-system foundation with a capability-oriented process model, real hardware bring-up, storage, graphics and a native desktop.

The original long-term concept is a polymorphic kernel capable of hosting multiple Personalities. That remains the architectural direction, but the current codebase should not be described as a finished Linux/Android/Windows compatibility system.

## 2. Current architectural layers

    Boot / x86_64
          |
          v
    Kernel core
      |    |    |
      |    |    +-- Syscalls / Ring3
      |    +------- Processes / scheduler
      +------------ Memory / paging
          |
          +---- Capability subsystem
          |
          +---- Device / hardware bring-up
          |       +-- PCI
          |       +-- AHCI/storage
          |       +-- Intel Gen6 graphics
          |       +-- HDA audio
          |       +-- Intel Wi-Fi
          |
          +---- Filesystem / storage
          |
          +---- Native desktop
                  +-- Window manager
                  +-- Terminal
                  +-- Explorer
                  +-- Settings
                  +-- Calculator
                  +-- Alarm
                  +-- Media Player

    Future Personality layer
          +-- Linux
          +-- Android
          +-- Windows

## 3. Kernel core

The kernel owns the low-level resources:

- physical memory;
- virtual memory/page tables;
- process address spaces;
- CPU execution state;
- syscalls;
- capability tables;
- device access;
- interrupts/DMA as driver infrastructure develops.

The current scheduler is cooperative/basic. It is not yet the final SMP/preemptive scheduler.

## 4. Process model

A process contains a Personality ownership identifier.

The intended invariant is:

> one process -> exactly one Personality for its lifetime.

The process cannot silently change ecosystem ABI while running.

The current code implements the ownership/lifecycle foundation; complete personality-specific execution environments are still future work.

## 5. Capability model

Capabilities are intended to be the kernel's authority primitive.

Current infrastructure includes:

- per-process capability tables;
- rights;
- validation;
- derivation;
- revocation;
- syscall-side checks for implemented objects.

The model is not yet complete enough to claim that every kernel resource is capability-only.

## 6. Personality model

A Personality is intended to provide an execution ABI for a specific ecosystem.

Conceptually:

    Personality
     +-- executable format
     +-- syscall ABI
     +-- process/runtime rules
     +-- ecosystem objects
     +-- private implementation state

The current Personality Manager provides IDs, load/unload state and process attachment tracking.

It does not yet load a real external Linux/Android/Windows runtime module.

## 7. Storage model

Current storage path is layered approximately as:

    Application / Explorer
            |
    Filesystem (AetherFS/FAT/NTFS)
            |
    Storage/partition layer
            |
    AHCI / block hardware
            |
    SATA device

The final VFS/device interfaces are still being stabilized.

## 8. Graphics model

The stable path currently begins with firmware/Multiboot framebuffer ownership and feeds the native desktop.

Intel Gen6 work adds guarded MMIO/register and display-pipeline bring-up.

The project has not yet reached a complete independent Intel modeset + accelerated renderer.

## 9. Audio model

The intended path is:

    Media source
       |
    WAV / MP3 decode
       |
    PCM buffer
       |
    HDA stream
       |
    BDL/DMA
       |
    HDA codec
       |
    physical output

The source/decoder side exists. The end-to-end hardware path is still under bring-up.

## 10. Network model

Intel 2230 bring-up currently concentrates on firmware/command/TX transport. Scan, association and a complete network stack are not yet complete.

## 11. Design invariants

These remain architectural goals:

- native CPU execution;
- no CPU instruction emulation;
- no Linux/Windows host OS underneath Aether;
- no classic VM as the primary compatibility mechanism;
- process ownership by one Personality;
- capability-oriented resource authority;
- lazy loading of Personality implementation where technically possible.

## 12. Current vs target

| Layer | Current | Target |
|---|---|---|
| x86_64 kernel | Implemented | Hardened |
| Memory/paging | Implemented | Hardened |
| Ring3 | Partial/implemented paths | Stable ABI |
| Scheduler | Cooperative/basic | Preemptive + SMP |
| Capabilities | Partial | Complete object authority |
| Storage | Working/partial | Unified VFS/device model |
| Graphics | Working desktop + Gen6 bring-up | Native modeset + acceleration |
| Audio | HDA bring-up | Verified PCM playback |
| Wi-Fi | Bring-up | Complete networking |
| Personality Manager | Foundation | Real loadable modules |
| Linux personality | Planned/partial foundation | Native compatibility layer |
| Android personality | Planned | Native Android runtime layer |
| Windows personality | Planned | Native PE/NT/Win32 layer |

## 13. What Aether is not

Aether is not:

- a Linux distribution;
- a Windows shell;
- a Linux subsystem running on another host OS;
- a QEMU-like CPU emulator;
- a collection of mock UI screens presented as an OS.

The current desktop is part of the native Aether runtime, while compatibility Personalities remain future architecture.
