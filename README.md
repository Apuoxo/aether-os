# Aether OS

**Native x86_64 Rust operating system — hardware-tested development project**

Aether is a clean-slate operating system written in Rust (no_std) with architecture-dependent assembly. It boots as a native x86_64 system rather than as a Linux distribution, Windows shell, CPU emulator, or hosted virtual machine.

The repository contains a working kernel and a growing native userspace, with real-hardware bring-up on a Fujitsu LIFEBOOK AH532.

## Current state — 2026-09-29

Aether has moved well beyond the design-only stage. The current tree contains working or partially working foundations for:
- native x86_64 boot and kernel runtime;
- physical memory allocation and paging;
- process/address-space and Ring3 ELF foundations;
- cooperative process scheduling;
- capability tables, validation, derivation and revocation primitives;
- native desktop/window management, cursor, keyboard and mouse input;
- terminal and native applications including Explorer, Calculator, Settings, Alarm and Media Player;
- RAM/AetherFS, FAT/NTFS, partition/block/storage layers and AHCI probing;
- Multiboot framebuffer graphics and Intel Gen6 / HD 3000 hardware bring-up;
- HDA controller discovery, MMIO, reset and codec infrastructure;
- MP3 decoding and the native media-player pipeline;
- Intel Centrino Wireless-N 2230 firmware/transport bring-up;
- GitHub Actions build and smoke-test infrastructure.

## Real hardware

The primary hardware validation target is a **Fujitsu LIFEBOOK AH532**.

Recorded hardware targets include:
- Intel HD Graphics 3000 — PCI `8086:0116`;
- Intel Centrino Wireless-N 2230 — PCI `8086:0887`, subsystem `8086:4062`;
- Realtek RTL8168 Ethernet — `10ec:8168`;
- Intel HDA controller — current AH532 playback bring-up target.

Hardware-dependent features are only considered verified when supported by actual AH532 evidence.

## Working vs. bring-up

### Kernel and userspace
Implemented / active: native x86_64 kernel; paging and physical-memory management; process and Ring3 ELF infrastructure; capability infrastructure; cooperative scheduler; native desktop and GUI; native applications.

Still being stabilized: final syscall/application ABI; deterministic Ring3 regression coverage; complete object-authority capability model; preemptive/threaded scheduler; unified VFS/AetherFS architecture.

### Graphics
The stable framebuffer desktop path works on the AH532, and Intel Gen6 register/KMS bring-up exists. It is not yet a complete production Intel graphics driver. Safe GGTT/GSM initialization, hardware acceleration, complete power management and fully verified native display initialization remain unfinished.

### Audio / Media Player
The HDA controller is detected and the native MP3/media pipeline exists. The remaining hardware blocker is **end-to-end PCM playback through the HDA stream and AH532 speakers**. Controller detection or a visible player window is not treated as proof of audible playback.

### Wi-Fi
Intel 2230 firmware loading, ALIVE handling, command infrastructure and transport bring-up exist. The current blocker is proving scheduler/SCD/TFD consumption and completing scan/network-result handling. Working firmware is therefore not described as working Wi-Fi networking.

## Long-term architecture

Aether's long-term design is a capability-oriented **polymorphic native kernel** with Personalities for different application ecosystems.

The intended direction is:

`Aether Kernel -> Personality Manager -> Linux / Android / Windows Personality`

A process belongs to exactly one Personality. Personalities are intended to be loadable and reclaimable when no process depends on them.

**This is the architectural target, not the current implementation.**

The repository does not currently contain:
- a complete Linux compatibility Personality;
- a complete Android compatibility Personality;
- a complete Windows/NT compatibility Personality;
- a real external hot-unloadable Personality module system;
- CPU instruction emulation.

## Current engineering priorities

1. Stabilize kernel/process/syscall/capability foundations.
2. Establish cleaner common device/driver/resource boundaries.
3. Finish storage/VFS boundaries.
4. Bring HDA PCM playback to verified AH532 speaker output.
5. Continue Intel 2230 SCD/TFD transport and scan bring-up.
6. Make Personality loading/unloading real.
7. Move from the current cooperative scheduler toward the planned preemptive/threaded model.

## Documentation and evidence

**Code is the source of truth for current behavior. Tests and real-hardware logs provide verification. Documentation explains the implementation and records intended future work.**

Start here:
- `docs/STATUS.md` — current engineering status;
- `docs/architecture/overview.md` — implemented architecture;
- `docs/design/personality_system.md` — Personality design and current limits;
- `docs/VIDEO_DRIVER_STATUS.md` — Intel Gen6 status;
- `docs/MEDIA_PLAYER.md` — Media Player/HDA status;
- `docs/network/WIFI_AH532.md` — Intel 2230 status;
- `docs/roadmap/` — historical and planned roadmap material.

For build evidence, use the **source commit SHA + GitHub Actions run + generated ISO artifact**. A build number or ISO is not considered evidence for a different source commit.

## Repository principle

Aether is being developed incrementally on real hardware. One controlled change, one verified build, then hardware testing where required.

Old audit documents and roadmap snapshots are retained as engineering history; they do not override the current source tree or `docs/STATUS.md`.

## License

The repository does not currently declare a final project license.