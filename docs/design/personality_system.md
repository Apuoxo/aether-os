# Personality System

## Status

**Architecture foundation: implemented. Full loadable compatibility personalities: not implemented.**

This document defines the target design while explicitly identifying what exists today.

## 1. Purpose

A Personality provides the ABI and runtime rules required to execute programs from a specific ecosystem.

Examples:

- Linux;
- Android;
- Windows.

A Personality is not intended to be a VM and does not emulate CPU instructions.

## 2. Hard requirements

A completed Personality system must satisfy:

1. A process belongs to exactly one Personality.
2. Personality loading is lazy.
3. Personality-owned code/data can be released when no process uses it.
4. Unloading is forbidden while live processes still depend on the Personality.
5. Personality-specific syscall dispatch is isolated from unrelated Personalities.
6. A failed Personality load cannot corrupt the base kernel.
7. Resource authority is represented through kernel capabilities.

## 3. Current implementation

The repository currently contains:

- PersonalityId values for Linux, Android and Windows;
- Personality Manager state;
- load/unload operations at manager level;
- process attach/detach lifecycle;
- process ownership of a Personality ID;
- unload protection while processes remain attached.

This is management infrastructure.

It is not yet a real dynamically loaded binary module system.

## 4. Target module

The eventual module can conceptually contain:

    Personality Module
    +-- header / ABI version
    +-- Personality ID
    +-- code
    +-- read-only data
    +-- syscall dispatch table
    +-- executable-format support
    +-- lifecycle hooks
    +-- private state

The exact binary format is intentionally not frozen yet.

## 5. Required kernel boundary

The Personality must not gain unrestricted kernel access.

Target interface:

    Personality
         |
         v
    stable kernel ABI
         |
         v
    capabilities / IPC / memory / scheduler / VFS / devices

This keeps the kernel independent of any one ecosystem.

## 6. Process lifecycle

Target:

    personality_acquire(id)
            |
    process_create(personality)
            |
           run
            |
    process_exit()
            |
    personality_release()
            |
    if refcount == 0:
        unload()

The current process attach/detach machinery provides the reference-counting direction but does not yet prove full memory reclamation.

## 7. Linux Personality

### Target

- ELF execution;
- Linux syscall ABI;
- POSIX-compatible foundation;
- Linux process/runtime semantics sufficient for selected native Linux binaries.

### Current status

The kernel has an ELF/Ring3 execution foundation, but this must not be described as Linux compatibility.

## 8. Android Personality

### Target

- Android process/runtime semantics;
- Binder;
- Android-specific permissions;
- ART integration;
- Android init/service model.

### Current status

Architecture target only.

## 9. Windows Personality

### Target

- PE loader;
- NT-style system-call/runtime boundary;
- Win32 compatibility layer;
- selected native Windows application execution.

### Current status

Architecture target only.

## 10. Unload acceptance test

A completed Personality implementation must have an evidence-based test that demonstrates:

1. Personality memory allocation before launch;
2. process attachment;
3. execution;
4. process exit;
5. Personality reference count reaches zero;
6. Personality-owned pages are released;
7. module registration disappears;
8. another Personality can subsequently load.

A log line saying unloaded=true is not sufficient evidence.

## 11. Current rule

Do not add new claims of Linux/Android/Windows compatibility until the corresponding executable and syscall/runtime tests exist.
