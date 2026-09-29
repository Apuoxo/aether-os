# Aether application contract — native app stage

## Current status

The Ring3/native-app path is implemented as an experimental kernel-mediated ABI. It is not yet a stable frozen application ABI.

## Current source boundary

- kernel/src/elf.rs loads x86_64 ELF images into a separate user address space.
- kernel/src/process.rs creates process records from loaded images.
- kernel/src/input.rs provides the kernel input event queue.
- kernel/src/gui/ and kernel/src/graphics.rs provide kernel-side drawing primitives.
- kernel/src/main.rs enters the built-in userspace path.

## Experimental syscall surface

The current int 0x80 path exposes:
- 10 — poll keyboard event
- 11 — draw NUL-terminated text
- 12 — fill rectangle
- 13 — yield
- 60 — process exit

The active register ABI is implemented by the current assembly/handler path and remains experimental until deterministic Ring3 regression evidence is complete. Display calls remain kernel-mediated; applications do not receive direct framebuffer/MMIO access.

## Calculator

Calculator is packaged as a separate native ELF from Apuoxo/aether-apps and is launched as an independent process after /bin/init.

The historical instruction not to expose guessed syscall numbers is superseded by the implemented experimental ABI above. It remains valid that new application interfaces must be defined in source, tested, and documented here.

## Remaining work

- deterministic syscall argument/return-value regression;
- deterministic CPL3 -> kernel entry/TSS evidence;
- stable application ABI definition;
- broader userspace resource/capability model.
