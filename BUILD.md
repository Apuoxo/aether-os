# Aether OS — build and evidence

Aether is a native x86_64 Rust `no_std` operating-system project. The repository also contains NASM/assembly and build tooling.

## Build dependencies

The main kernel ISO workflow uses the repository's Makefile and GitHub Actions. The expected toolchain includes:

- nasm
- rustc / cargo-compatible Rust tooling used by the kernel build
- binutils / ld
- grub-mkrescue
- xorriso

The authoritative build procedure is `.github/workflows/build.yml` plus `kernel/Makefile`. This document is a short orientation, not a replacement for the workflow.

## Evidence chain

A testable ISO must be identified by all of:

1. source commit SHA;
2. GitHub Actions run;
3. generated `aether.iso` artifact;
4. embedded build identity when available.

A successful source commit is not, by itself, proof that an ISO artifact corresponds to that commit.

## Current major modules

- `kernel/src/drivers/ahci.rs` — AHCI/storage hardware layer
- `kernel/src/fs_ntfs.rs` — NTFS read/mount path
- `kernel/src/fs_fat.rs` — FAT support
- `kernel/src/files_mgr.rs` — Explorer/files manager
- `kernel/src/desktop.rs` — native desktop and GUI terminal dispatch
- `kernel/src/drivers/audio.rs` — HDA/PCM bring-up
- `kernel/src/media_player.rs` — native media player
- `kernel/src/drivers/wifi.rs` — Intel 2230 bring-up
- `kernel/src/drivers/intel_kms.rs` / `video.rs` — Intel Gen6 display bring-up
- `kernel/src/process.rs` / `elf.rs` — process and Ring3 ELF foundation

## Hardware rule

QEMU/build success is laboratory evidence only. Hardware-dependent claims require a real-AH532 test and must be recorded in the engineering log.
