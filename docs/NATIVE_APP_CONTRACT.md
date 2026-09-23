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


## Implemented syscall surface (experimental)
The kernel now exposes the first native-app calls on `int 0x80`:
- `10` — poll keyboard event
- `11` — draw NUL-terminated text
- `12` — fill rectangle
- `13` — yield
- `60` — process exit

The display calls remain kernel-mediated: applications do not receive direct framebuffer/MMIO access. The framebuffer is prepared before Ring3 using the existing Multiboot surface path; the Intel display driver remains read-only and no GGTT/GSM mapping is introduced.

Calculator is packaged as a separate native ELF from Apuoxo/aether-apps and is launched as an independent process after `/bin/init`.
