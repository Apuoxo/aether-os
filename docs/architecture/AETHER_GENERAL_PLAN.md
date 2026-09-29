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

## 5. Windows Personality — first target

Windows is the first Personality because it gives Aether a concrete end-to-end binary-compatibility milestone.

Initial path:

```
real Windows PE
    -> PE loader
    -> Windows process/thread model
    -> NT/Win32 compatibility
    -> Aether kernel objects/VFS/IPC/graphics
    -> Aether compositor/input
```

The first acceptance target is a small real Win32 application, not a native Aether imitation.

Milestones:

1. PE header/parser.
2. PE section mapping with correct R/W/X permissions.
3. Relocations.
4. Import resolution.
5. Windows process bootstrap.
6. NTDLL/NT compatibility foundation.
7. Kernel32-compatible subset.
8. User32-compatible window/message subset.
9. GDI-compatible drawing subset.
10. Aether input -> Windows message translation.
11. Real Win32 test application.
12. Resource cleanup and Personality unload.

A Windows environment image/WIM/WinPE-style environment may be used as an **Environment Pack** providing DLLs, resources, fonts and other user-space components.

It is an environment source, not a second Windows kernel.

Modern WinUI/Store applications and complex DirectX software are later compatibility targets, not prerequisites for the first proof.

## 6. Linux Personality — second target

Linux Personality follows the stabilization of the common Aether foundations and the first Windows vertical slice.

Priority areas:

- ELF execution;
- Linux process/thread semantics;
- virtual memory;
- file descriptors;
- signals;
- futexes;
- pipes/IPC;
- sockets;
- dynamic linking/runtime;
- Linux filesystem semantics.

LKL and other reusable Linux components should be evaluated before implementing large subsystems independently.

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

The first Windows milestone is considered complete only when a real PE application has:

- loaded successfully;
- created a real Aether-backed process/thread;
- resolved its required APIs;
- created a real window;
- accepted keyboard/mouse input;
- rendered through the Aether graphics path;
- exited cleanly;
- released all resources.

## 11. Architectural decision

This document records the selected general direction:

> **Aether remains the native kernel. Windows, Linux and Android become loadable/reclaimable Personalities above a stable Aether kernel ABI. Existing OS components may be adapted where they materially reduce implementation cost and fit the Aether object/resource model. Complete foreign kernels are not embedded as the normal execution model.**

This is the target architecture. Current implementation status must continue to be reported separately in `docs/STATUS.md` and source code.
