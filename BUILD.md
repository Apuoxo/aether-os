# Aether OS — sources

Bare-metal x86_64 microkernel (not Linux-based).

## Build deps
nasm, rustc, binutils (ld), grub-mkrescue, xorriso

## Build
See comments in kernel/src/main.rs and kernel/Makefile if present.
Typical:
  nasm boot.s/isr.s -> .o
  rustc --emit=obj main.rs (no_std, static)
  ld -T linker/linker.ld -> aether.bin
  grub-mkrescue -> ISO

## Important modules
- drivers/ahci.rs — AHCI READ-ONLY
- fs_ntfs.rs — NTFS RO root list
- fs_fat.rs, fs.rs — FAT / AetherFS
- desktop.rs — XP-style desktop
- process.rs, arch/ — Ring3 / syscall
