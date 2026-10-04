# Aether Laboratory Status

**Status basis:** source code + CI evidence + documented real-hardware tests.  
**Last reviewed:** 2026-09-29.

This is the current engineering status. It deliberately separates implemented infrastructure from incomplete or planned functionality.

## 1. Executive status

Aether is a bootable native x86_64 Rust operating system in active hardware development.

It is no longer a design-only kernel. The repository contains a working kernel/userspace foundation, storage stack, graphics/desktop stack, applications and several real hardware-driver bring-up paths.

The long-term polymorphic Personality architecture is present as a kernel foundation, but Linux, Android and Windows are not yet complete compatibility personalities.

## 2. Kernel foundation

### Implemented / present

- x86_64 native kernel runtime;
- physical page allocation;
- paging;
- separate user/kernel address spaces;
- Ring3 ELF execution infrastructure;
- process objects;
- cooperative scheduler;
- process-owned Personality ID;
- syscall ABI infrastructure;
- per-process capability tables;
- capability validation, derivation and revocation primitives;
- kernel serial diagnostics.

### Incomplete

- deterministic full syscall/TSS runtime evidence for all paths;
- complete object-authority capability model;
- preemptive scheduler;
- full thread model;
- SMP;
- complete IPC subsystem;
- final stable syscall ABI coverage.

## 3. Storage

### Implemented / present

- AetherFS/RAM storage path;
- FAT support;
- partition/storage diagnostics;
- AHCI probing/infrastructure;
- NTFS mounting and directory visibility on the tested AH532 environment;
- Explorer integration.

### Incomplete

- final VFS/device boundary;
- complete write support and recovery semantics across all filesystems;
- comprehensive storage regression suite.

## 4. Graphics and desktop

### Verified on target hardware

Target: Fujitsu LIFEBOOK AH532, Intel HD Graphics 3000, PCI 8086:0116, Gen6.

Implemented:

1. Multiboot framebuffer;
2. dynamic framebuffer geometry;
3. Intel display PCI discovery;
4. guarded BAR0 MMIO access;
5. Gen6 register diagnostics;
6. framebuffer/scanout comparison infrastructure;
7. GMBUS/EDID diagnostic path;
8. desktop/window manager;
9. cursor and mouse handling;
10. localized window redraws for input/mouse updates;
11. terminal and multiple native desktop applications.

### Incomplete

- independent cold-boot native Intel modeset;
- safe GGTT/GSM initialization;
- hardware-accelerated rendering;
- complete connector/power management;
- verified real-AH532 EDID acquisition;
- complete production Intel display driver.

Safety constraint: previous GGTT/GSM mapping experiments rebooted the AH532. The stable framebuffer path must remain intact until the mapping is understood.

Detailed display status: docs/VIDEO_DRIVER_STATUS.md.

## 5. Desktop applications

Current native desktop/application code includes:

- Terminal;
- Explorer;
- Calculator;
- Settings;
- Alarm;
- Media Player.

These applications are real kernel-integrated/native components, but individual features may still depend on unfinished storage, audio, networking or GUI infrastructure.

## 6. Audio

### Implemented / present

- HDA PCI discovery;
- BAR0 MMIO mapping;
- HDA reset sequence;
- codec state discovery;
- CORB/RIRB transport;
- codec verb diagnostics;
- stream/PCM data structures;
- PCM buffering;
- WAV parsing;
- MP3 decoding;
- Media Player control path;
- volume/mute/repeat/shuffle/seek state.

### Current blocker

End-to-end HDA speaker playback is not yet verified on the AH532.

The important distinction is:

HDA controller detected != PCM stream running != audible playback.

Current work must concentrate on:

HDA stream setup -> BDL/DMA -> RUN -> LPIB progress -> codec output -> audible PCM.

## 7. Wi-Fi

Target: Intel Centrino Wireless-N 2230, PCI 8086:0887.

Current state:

- PCI discovery: implemented;
- BAR/MMIO: implemented;
- firmware contract/firmware loading path: implemented to bring-up level;
- ALIVE: observed;
- command queue: initialized;
- TX/SCD consumption: not yet proven;
- scan/association/network stack: not complete.

Current diagnostic focus is the SCD/TFD ownership/consumption path rather than a general driver rewrite.

## 8. Personality system

### Implemented foundation

- PersonalityId values for Linux, Android and Windows;
- process ownership of a Personality ID;
- attach/detach lifecycle hooks;
- Personality Manager state;
- unload protection based on process ownership.

### Not yet implemented as promised by the architecture

- external/loadable personality binary format;
- executable personality code/data sections;
- real syscall table registration per personality;
- complete memory reclamation of personality modules;
- Linux syscall compatibility layer;
- Android Binder/ART/runtime integration;
- Windows PE/NT compatibility layer.

Therefore the correct status is:

**Personality architecture: foundation implemented.  
Full personalities: not implemented.**

## 9. Scheduler

The current scheduler is cooperative/basic.

This is sufficient for the present bring-up environment but is not the final scheduler model.

Future work:

- kernel threads;
- timer-driven preemption;
- context switching;
- SMP;
- CPU affinity;
- scheduler accounting.

## 10. AI / persistent development infrastructure

The repository contains groundwork for future native development/agent functionality.

It is not currently a core dependency of boot, scheduling, storage or desktop operation.

The project must not describe Aether as an AI operating system until an actual native AI subsystem is implemented and verified.

## 11. Hardware evidence

The tested AH532 remains authoritative over generic hardware documentation.

The repository also contains the verified FH6/FH6C HM70-family schematic:

- 45 pages;
- SHA-256 8011b1775a403b5d62ff816c278412f62c2e3d7873b955f2721fa664528badbc.

It is used for board-family signal routing, not as proof of the exact installed GPU.

## 12. CI / build identity

CI is the reproducible build gate.

The repository must treat:

- Git commit SHA;
- GitHub Actions run;
- generated ISO/artifact;
- embedded Aether build identity

as one evidence chain.

The current kernel contains historical build/version strings that are not sufficient to identify an ISO uniquely. A future build-identity cleanup must expose the exact commit/run identity inside the OS and artifact metadata.

## 13. Current priority order

### P0 — kernel correctness

- deterministic Ring3/syscall/TSS evidence;
- capability enforcement;
- process/thread foundation.

### P1 — device architecture

- common device/resource/IRQ/DMA model;
- clean driver binding.

### P1 — audio

- finish HDA PCM DMA playback;
- verify audible playback on AH532;
- then close the Media Player hardware dependency.

### P1 — networking

- prove Wi-Fi TX/SCD consumption;
- RX;
- scan;
- association;
- network stack integration.

### P2 — VFS/storage

- stabilize filesystem/device boundaries;
- expand NTFS regression coverage.

### P2 — Personality

- turn the manager foundation into a real loadable personality mechanism;
- then implement the first actual personality.

## 14. Definition of reality

Use these status words consistently:

- **Verified** — reproduced by an explicit test and evidence exists.
- **Implemented** — source implementation exists, but hardware/runtime proof may still be incomplete.
- **Bring-up** — actively being made functional on hardware.
- **Partial** — meaningful implementation exists but required pieces are missing.
- **Planned** — design/roadmap only.
- **Blocked** — a known technical blocker prevents completion.
- **Historical** — retained for record; not current behavior.

Never mark a feature Verified merely because a source file or UI exists.

- SMP: opt-in AH532 multi-AP long mode 4/4; CPU1/CPU6/CPU19/CPU20 validation path.
