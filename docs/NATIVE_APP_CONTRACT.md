# Aether application contract — first native app stage

This document records the currently implemented boundary between the kernel and future native applications.

## Evidence from source

- kernel/src/elf.rs loads x86_64 ELF ET_EXEC images into a separate user CR3.
- kernel/src/process.rs creates process records from loaded images.
- kernel/src/input.rs already has a kernel input event queue.
- kernel/src/gui/ and kernel/src/graphics.rs provide kernel-side drawing primitives.
- kernel/src/main.rs currently enters the built-in /bin/init ELF path, but the Ring3 path is still a boot-stage mechanism rather than a stable application ABI.

## Decision

Do not expose guessed syscall numbers or direct kernel GUI calls to Calculator yet.

The next kernel milestone is a minimal, explicit native-app ABI with:
1. process exit;
2. keyboard event polling;
3. basic text drawing;
4. basic rectangle/button drawing;
5. a safe yield/wait primitive.

The ABI must be implemented and tested before aether-apps/apps/calculator/src/main.rs is connected to it.

## Calculator target

Calculator remains split into a pure no_std arithmetic engine, a thin Aether ABI adapter, and a GUI/input layer. This keeps arithmetic testable and prevents the app from depending on kernel internals.
