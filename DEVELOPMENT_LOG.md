# Aether OS Development Log

This file is the persistent engineering log for the Aether OS repository.

## Rules

- Do not claim a subsystem is complete without runtime evidence.
- Record source changes, build results, hardware results, failures, and the next controlled step.
- Preserve known-good states and avoid repeating experiments already shown unsafe.
- AH532 is a real-hardware validation target; QEMU success is not equivalent to AH532 success.
- Current graphics hardware: Intel Sandy Bridge HD Graphics 3000, PCI 8086:0116, Gen6.
- GGTT/GSM physical region 0xDF800000 is currently considered unsafe on AH532 after mapping attempts caused reboot. Do not re-enable such mapping without a new, justified test plan.

## Current State — 2026-09-23

### Build / CI

- Added the missing NASM isr.o object to the kernel link.
- An attempted switch to x86_64-unknown-none caused freestanding-link panic-symbol problems (core::panicking::panic_bounds_check, panic_const_div_by_zero).
- Restored the existing compatible Rust target: x86_64-unknown-linux-gnu.
- Removed the now-unused x86_64-unknown-none target installation from CI.
- Latest corrective commits: b3ede6e0224679647b1896edd5bb001ccedce460 and 499bdceef4a7e1225962e82a8a9a3cd784ed7b1a.
- Verification must be based on the observed GitHub Actions result, not the commit message alone.

### Video

- KMS/modeset on AH532 is already experimentally confirmed.
- 800x600 desktop is stable; the previous mouse geometry problem was fixed by using dynamic framebuffer dimensions.
- Confirmed Intel Gen6 pipe/vblank diagnostics.
- MMIO and KMS path are usable.
- GGTT/GSM mapping attempts around physical 0xDF800000 caused an AH532 reboot and remain disabled.
- A safe shell diagnostic command vdiag was added to inspect video state without mapping GGTT/GSM.
- Current source also contains explicit read-only scanout and EDID/GMBUS paths.
- Real-AH532 EDID success is not yet proven.
- Detailed status is maintained in docs/VIDEO_DRIVER_STATUS.md.

### Archive and hardware-document audit — 2026-09-23

- aether-os-sources.zip was unpacked successfully through GitHub Actions.
- The archive is a historical source snapshot; its video.rs is only the old software-framebuffer wrapper.
- The current repository video driver is substantially newer and must not be overwritten by the archive.
- The uploaded DA0FH6MB6E0 rev E RAR was extracted successfully through GitHub Actions.
- Its PDF identifies an Intel Calpella / Arrandale UMA platform with HM55 PCH and project FH2, not the AH532 Sandy Bridge/HM76 platform.
- The PDF is therefore classified as REFERENCE-MISMATCH for AH532 hardware design.
- It must not be used for AH532 GPU register, power, GPIO, display-routing, or pinout decisions.
- Repository documentation was updated to record this distinction.
- The next graphics step remains controlled read-only EDID/GMBUS validation; no GGTT/GSM mapping or automatic modeset should be introduced as part of that test.

### Working discipline

Every significant change should be followed by:
1. Source inspection.
2. Build/CI verification.
3. Real-hardware test when hardware behavior is involved.
4. A short entry here describing the result and next step.

## History

### 2026-09-23 — Safe video diagnostics
- Added vdiag shell command.
- Intended scope: software framebuffer readiness, Intel Gen6 MMIO readiness, existing scanout attachment, pipe/surface state, and existing read-only video snapshot.
- No GGTT/GSM mapping and no display-register mutation.

### 2026-09-23 — CI/linker repair
- Identified that src/arch/x86_64/isr.s exists and exports symbols required by the kernel.
- Added NASM assembly of isr.s to kernel/Makefile and linked isr.o.
- First target migration to x86_64-unknown-none did not work with the current freestanding source/link setup.
- Reverted to the repository's previous x86_64-unknown-linux-gnu target and removed the unused target installation from workflow.

## Do Not Repeat Without New Evidence

- Do not map or directly access the AH532 GGTT/GSM physical region 0xDF800000.
- Do not treat KMS=READY as proof that the full Intel graphics driver is complete.
- Do not treat a successful QEMU build/boot as proof of AH532 graphics correctness.
- Do not proceed to broad driver expansion while the CI build is broken.

## Calculator native app pipeline (2026-09-23)

- Added experimental Ring3 syscall surface for keyboard polling, kernel-mediated text/rectangle drawing, yield, and process exit.
- Added PS/2 polling bridge so the known-good AH532 keyboard path reaches native apps; USB HID events continue through the existing input queue.
- Prepared the existing Multiboot framebuffer before Ring3 without adding any GGTT/GSM mapping or display-register writes.
- Added separate Apuoxo/aether-apps Calculator ELF build pipeline and Aether OS CI packaging path.
- Calculator is launched as an independent ELF process after /bin/init; shell remains the fallback/next stage.
- Runtime status: NOT YET VERIFIED on QEMU or AH532. CI success alone is insufficient.

## QEMU Calculator evidence (2026-09-23)

- Aether Apps Calculator CI builds a freestanding x86_64 ELF successfully and publishes apps/calculator/dist/calculator.elf.
- Aether OS CI packages that ELF into the kernel build without writing the Calculator image to AetherFS/host storage.
- QEMU smoke test passed: Calculator ELF loaded from the bundled package, PID=2 entered with USER_CR3 different from KERNEL_CR3, and syscalls 12 (rectangle), 11 (text), 10 (keyboard poll), and 13 (yield) were observed from CPL=3.
- No PANIC or user page-fault kill was observed in the smoke test.
- This is QEMU evidence only; AH532 hardware validation remains required, especially PS/2 keyboard interaction and the already-known Gen6 display path.


### 2026-09-24 — AH532 boot failure: GRUB video mode
- Real-hardware validation of build #139 on Fujitsu AH532 failed before Aether OS kernel startup with the reported message: "Error: no suitable video mode found".
- This is currently classified as a bootloader/GRUB video-mode failure, not evidence of a Ring3 or userspace failure.
- The published ISO was built successfully and its QEMU native-app smoke test passed; therefore the AH532 failure is a separate real-hardware boot-path issue.
- Source inspection shows kernel/Makefile generates a minimal GRUB config containing only the Multiboot2 entry and no explicit gfxmode/video mode.
- No code or graphics-driver changes were made in response to this report yet.
- Next controlled step: inspect the GRUB/Multiboot boot path and make the smallest change that allows AH532 to enter the kernel without assuming a firmware-supported graphics mode; then rebuild and retest QEMU before another AH532 run.


### 2026-09-24 — Controlled GRUB text-mode fix for AH532 boot failure
- Root symptom under real AH532: GRUB reported "Error: no suitable video mode found" before kernel startup.
- Source inspection found the ISO Makefile generated a minimal GRUB configuration without explicit text-mode/gfxpayload settings.
- Applied the smallest bootloader-side change: GRUB now requests `gfxmode=text`, `gfxpayload=text`, and `terminal_output console` before the Aether Multiboot2 entry.
- No kernel, Intel driver, framebuffer ownership, GGTT/GSM, Ring3, storage, or Wi-Fi code was changed.
- Commit: 3c60f7ca5cabd61346b5077542a6123a288b6455.
- Runtime result: not yet verified. Required next step is GitHub CI build + QEMU smoke test, then a fresh ISO must be tested on AH532.
