# Aether OS — General Architecture Plan

**Status:** Target architecture / engineering plan  
**Date:** 2026-09-29

## 1. Purpose

Aether remains a native x86_64 Rust `no_std` operating system with its own kernel and direct hardware control.

The long-term goal is one Aether kernel that can host multiple application ecosystems through loadable/reclaimable **Personalities**, rather than booting or embedding complete foreign operating-system kernels.

The selected direction is a **hybrid Personality architecture**:

```
Aether Kernel
    |
    +-- Runtime Manager
          |
          +-- Windows Personality
          +-- Linux Personality
          +-- Android Personality
```

A process belongs to exactly one Personality.

## 2. What is deliberately NOT the architecture

Aether will not become:

- a Linux distribution;
- a Windows shell;
- a collection of three simultaneously running foreign kernels;
- a CPU emulator;
- a classic VM for every Personality;
- a system that requires rewriting every compatibility component from scratch.

A complete foreign kernel may be studied for reusable algorithms, interfaces, or adaptable components, but it is not the normal execution model of Aether.

## 3. Core structure

```
                         AETHER
                            |
                    Aether Native Kernel
                            |
        +-------------------+-------------------+
        |                   |                   |
       CPU                  MM                 IPC
    Scheduler            Objects              VFS
       IRQ              Capabilities         Drivers
        |                   |                   |
        +-------------------+-------------------+
                            |
                     Runtime Manager
                            |
          +-----------------+-----------------+
          |                 |                 |
       WINDOWS            LINUX            ANDROID
      Personality       Personality       Personality
          |                 |                 |
       PE/NT/Win32       ELF/Linux       APK/ART/Bionic
          |                 |                 |
          +-----------------+-----------------+
                            |
                    Stable Aether ABI
                            |
                         Hardware
```

Each Personality owns the semantics needed by its application ecosystem:

- executable loader;
- ABI/API surface;
- process and thread semantics;
- memory semantics;
- IPC semantics;
- filesystem/handle semantics;
- networking semantics;
- graphics/input integration;
- environment provider;
- compatibility components.

Common hardware, memory, objects, capabilities, scheduling, IPC and core VFS remain Aether responsibilities.

## 4. Reuse existing OS components

Aether should **reuse or adapt existing mature components where this is technically practical**, instead of automatically rewriting everything.

Examples to investigate:

- Linux/LKL-derived components for Linux Personality functionality;
- NT/Windows compatibility techniques demonstrated by projects such as NeptuneOS;
- existing standards-compatible runtime components where their integration model fits Aether.

The rule is:

> Reuse a component when it can be cleanly adapted to the Aether kernel ABI and object model. Do not import an entire foreign kernel merely because it already exists.

All reused code must be evaluated for architecture fit, licensing, maintenance cost and hardware/resource ownership.

## 5. Linux Personality — first target

Linux is the first executable-compatibility target because Aether already has an ELF64 loader, Ring3/process foundation, VFS storage path and Linux Personality scaffold. The first proof must be a real external Linux ELF loaded from the hard disk.

Initial path:

```
real Linux ELF on HDD
    -> Aether VFS
    -> Runtime Manager
    -> Linux Personality
    -> ELF loader
    -> Linux syscall ABI
    -> Aether kernel objects/VFS
```

The first acceptance target is an unmodified GNU Hello Linux x86_64 ELF stored on a mounted hard-disk filesystem.

Milestones:

1. External ELF format resolution.
2. VFS path -> image read.
3. ELF segment mapping with correct R/W/X permissions.
4. Linux process bootstrap.
5. Linux write/exit syscall subset.
6. Process cleanup and Personality release.
7. Static file-I/O application.
8. Dynamic ELF/runtime support.
9. Threads/futex/signals.
10. Sockets/networking.
11. Real Linux GUI application.
12. Large application such as Firefox.

A Windows environment image/WIM/WinPE-style environment may be used as an **Environment Pack** providing DLLs, resources, fonts and other user-space components.

It is an environment source, not a second Windows kernel.

Modern WinUI/Store applications and complex DirectX software are later compatibility targets, not prerequisites for the first proof.

## 6. Windows Personality — second target

Windows Personality follows the first Linux vertical slice and the stabilization of the common executable/process foundations.

Priority areas:

- PE execution;
- relocations and imports;
- NT process/thread semantics;
- Win32 DLL compatibility;
- User32/GDI windowing;
- input/message translation;
- Windows environment packs.

NeptuneOS-style NT compatibility techniques and reusable components should be evaluated before implementing large subsystems independently.

## 7. Android Personality — third target

Android is not treated as simply another Linux executable format.

Required direction includes:

- APK/ELF execution;
- Bionic;
- Binder on top of generic Aether IPC;
- ART/runtime;
- Android process/permission semantics;
- property/service infrastructure;
- system services;
- Android environment;
- graphics/input integration.

Android comes after the generic IPC, memory, VFS, process/thread and graphics foundations are mature enough.

## 8. Common kernel gates before large Personality work

The following foundations are mandatory engineering gates:

1. **Kernel execution:** boot, paging, PMM, GDT/IDT/TSS, stable Ring3.
2. **Process/thread:** real threads, context switching, preemption, blocking/wakeup, timer support.
3. **Memory:** VMA, mmap/munmap/mprotect, page-fault recovery, COW, shared memory, correct R/W/X.
4. **Kernel objects:** Process, Thread, File, Directory, Memory, Device, Socket, Event, Mutex, Semaphore, Timer, IPC Endpoint, SharedMemory.
5. **Capabilities:** capability handles reference kernel objects with rights and generation protection.
6. **IPC:** endpoints, messages, blocking/wakeup, shared memory and handle transfer.
7. **VFS:** stable file/directory/device/descriptor model.
8. **Executable subsystem:** generic loader framework plus ELF and PE implementations.
9. **Stable syscall/ABI path:** CPU entry -> generic dispatcher -> current Personality -> Personality ABI -> common kernel service.
10. **Security/resource control:** credentials, permissions, capabilities and resource limits.
11. **Networking:** NIC -> Ethernet -> IP -> UDP/TCP -> sockets -> DNS/routing/loopback.

## 9. Runtime lifecycle

The intended execution path is:

```
Application launch
    |
    v
Runtime Manager
    |
    +-- identify executable format
    |
    +-- resolve Personality
    |
    +-- acquire Personality
    |
    +-- create Environment
    |
    +-- invoke format-specific loader
    |
    +-- create process/thread
    |
    +-- bind Personality ABI
    |
    +-- run
    |
    +-- release process resources
    |
    +-- release Environment
    |
    +-- unload Personality when unused
```

Personality loading/unloading must become real and testable. A simple boolean state transition is not sufficient.

## 10. Engineering method

Development follows the established Aether rule:

**one controlled change -> one commit -> GitHub Actions verification -> hardware test when applicable -> next commit.**

No large speculative Personality implementation should be merged as one unverified change.

The first Linux milestone is considered complete only when a real external ELF application has:

- been loaded from a mounted hard-disk filesystem;
- created a real Aether-backed process;
- executed through the Linux syscall ABI;
- produced its expected output;
- exited cleanly;
- released all process resources;
- released the Linux Personality when its final process exits.

## 11. Architectural decision

This document records the selected general direction:

> **Aether remains the native kernel. Windows, Linux and Android become loadable/reclaimable Personalities above a stable Aether kernel ABI. Existing OS components may be adapted where they materially reduce implementation cost and fit the Aether object/resource model. Complete foreign kernels are not embedded as the normal execution model.**

This is the target architecture. Current implementation status must continue to be reported separately in `docs/STATUS.md` and source code.
