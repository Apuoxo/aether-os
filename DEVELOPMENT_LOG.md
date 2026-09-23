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
- Next verification: inspect the GitHub Actions run triggered by 499bdce...; do not assume success until the job result is observed.

### Video

- KMS/modeset on AH532 is already experimentally confirmed.
- 800x600 desktop is stable; the previous mouse geometry problem was fixed by using dynamic framebuffer dimensions.
- Confirmed Intel Gen6 pipe/vblank diagnostics.
- MMIO and KMS path are usable.
- GGTT/GSM mapping attempts around physical 0xDF800000 caused an AH532 reboot and remain disabled.
- A safe shell diagnostic command vdiag was added to inspect video state without mapping GGTT/GSM.
- Next video phase after CI is green: inspect/validate read-only scanout and EDID/GMBUS paths, with strict bounds and no risky GGTT/GSM access.

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
