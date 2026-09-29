# Aether OS

**Native x86_64 Rust operating system — current status: hardware-tested development build**

Aether is a clean-slate native operating system written in Rust (no_std) with architecture-dependent assembly. The current system is a real bootable kernel with userspace/Ring3 groundwork, storage, graphics, desktop, applications, and active hardware-driver bring-up.

The long-term architecture remains a capability-based polymorphic kernel with loadable Personalities (Linux, Android, Windows). That architecture is a target: the current repository does not yet provide full Linux/Android/Windows compatibility personalities.

## Current state

The repository is past the initial design-only stage.

Currently implemented in the codebase include:

- x86_64 boot and native kernel runtime;
- physical memory allocation and paging;
- separate user/kernel address spaces and Ring3 ELF execution infrastructure;
- process objects, a cooperative scheduler, and process-owned Personality IDs;
- per-process capability tables with validation, derivation and revocation primitives;
- Multiboot framebuffer graphics and an active Intel Gen6/HD 3000 bring-up path;
- desktop/window manager, cursor, keyboard and mouse handling;
- terminal, Explorer, Calculator, Settings, Alarm and Media Player applications;
- AetherFS/RAM storage, FAT/NTFS work, partition/block/storage layers and AHCI probing;
- HDA controller discovery/MMIO/reset/codec-verb infrastructure and a PCM playback path under active development;
- Intel Centrino Wireless-N 2230 firmware/transport bring-up under active development;
- MP3 decoder and native media streaming pipeline;
- GitHub Actions build/smoke-test infrastructure.

The system is tested on real hardware, especially a Fujitsu LIFEBOOK AH532 with Intel HD Graphics 3000 (PCI 8086:0116).

## Important distinction

The following are architecture goals, not completed compatibility layers:

- full Linux personality;
- full Android personality;
- full Windows personality;
- hot-unloadable external personality binaries;
- complete preemptive SMP scheduler;
- complete universal device/driver model;
- production-grade native Intel display acceleration;
- working Wi-Fi networking;
- fully verified HDA PCM speaker playback.

## Architecture direction

    AETHER KERNEL
         |
    +----+--------------------+
    |         |               |
  Memory  Capability       Process
    |         |               |
    +---------+---------------+
              |
       Device / IPC layers
              |
    +---------+---------+
    |         |         |
 Storage   Graphics  Drivers
              |
        Native Desktop
              |
         Applications
              |
       Personality layer
       /       |           Linux    Android   Windows

## Documentation rule

Source code and reproducible test evidence are authoritative for current behavior.

- docs/STATUS.md describes the current engineering state.
- docs/architecture/ describes implemented architecture and clearly marked future architecture.
- docs/design/ contains subsystem design constraints.
- docs/roadmap/ contains planned work and historical context.

Historical version labels must not be treated as proof that a feature is currently implemented.

## Near-term engineering priorities

1. Stabilize kernel/process/syscall/capability foundations.
2. Establish a common device/driver/resource model.
3. Finish storage/VFS boundaries.
4. Finish HDA PCM playback and validate the Media Player end-to-end.
5. Continue Intel Wi-Fi transport bring-up.
6. Make Personality loading/unloading real rather than manager-only state.
7. Replace the current cooperative scheduler with the planned preemptive/threaded model.

## License

The repository does not currently declare a final project license.
