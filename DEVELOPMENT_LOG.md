# Aether OS Development Log

## Current State — 2026-09-29

- Documentation maintenance is based on the actual main tree at commit 0bb996ed5ab809ae6c73709262ea964e7c3f3224.
- A full markdown inventory exposed several documents that still described superseded implementation stages.
- Current live status is maintained in docs/STATUS.md; historical audits/log entries remain evidence but are not current state.
- Current GUI terminal routing is in kernel/src/desktop.rs (run_cmd()).
- Current hardware blockers remain HDA PCM playback and Intel 2230 SCD/TFD transport consumption.
- The stable Intel framebuffer path remains protected from the known-unsafe GGTT/GSM region around 0xDF800000.


This file is the persistent engineering log for the Aether OS repository.

## Rules

- Do not claim a subsystem is complete without runtime evidence.
- Record source changes, build results, hardware results, failures, and the next controlled step.
- Preserve known-good states and avoid repeating experiments already shown unsafe.
- AH532 is a real-hardware validation target; QEMU success is not equivalent to AH532 success.
- Current graphics hardware: Intel Sandy Bridge HD Graphics 3000, PCI 8086:0116, Gen6.
- GGTT/GSM physical region 0xDF800000 is currently considered unsafe on AH532 after mapping attempts caused reboot. Do not re-enable such mapping without a new, justified test plan.

## Historical State — 2026-09-23

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


### 2026-09-24 — AH532 retry: GRUB text-mode settings did not resolve failure
- Real AH532 retest of build #142 still reports the same "Error: no suitable video mode found" before Aether startup.
- This disproves the previous assumption that adding runtime `gfxmode=text` and `gfxpayload=text` was sufficient.
- No kernel/runtime evidence has been obtained from this boot attempt; Ring3, Intel KMS, framebuffer, storage, and userspace are therefore not implicated by this symptom.
- Applied a narrower follow-up bootloader experiment: removed the `gfxmode` and `gfxpayload` directives and retained only `terminal_output console` in the generated GRUB configuration.
- Commit: 45a59d24f4b695f29f43d2849f3ac4a27f5c427d.
- No kernel, graphics-driver, GGTT/GSM, Ring3, storage, or Wi-Fi code was changed.
- Next step: verify CI/QEMU build, then test the resulting ISO on AH532. If the same GRUB error persists, inspect the GRUB image/modules and boot path rather than making further blind kernel changes.


### 2026-09-24 — AH532 retry #2: GRUB console-only config still fails
- Real AH532 retest of build #144 still reports the same "Error: no suitable video mode found" before Aether startup.
- Build #144 was independently inspected: the published artifact contains a 12,953,600-byte ISO, and its embedded grub.cfg contains only `set timeout=0`, `set default=0`, `terminal_output console`, and the Multiboot2 entry. This confirms the failure is not caused by the previously removed gfxmode/gfxpayload directives.
- GNU GRUB documentation confirms `terminal_output console` is a valid native console output and that `grub-mkrescue --install-modules=...` can restrict installed modules and their dependencies. citeturn1search0turn1search1
- Applied the next controlled bootloader experiment: restrict the ISO to the minimum GRUB modules needed for legacy BIOS ISO9660 + Multiboot2 + normal/terminal boot, and remove installed themes/fonts/locales. The kernel and runtime code are unchanged.
- Commit: 0c79ad9da5e698b5f70412dd6339f36384b6bcac.
- No claim of AH532 success yet. Required next step: CI build/QEMU smoke test, then fresh ISO on AH532. If the same error persists, the next investigation is the GRUB platform image/core path rather than further kernel changes.


### 2026-09-24 — Explorer architecture correction: logical volumes vs physical partitions
- Current task owned by Virt: finish the Windows 7-style Explorer so its UI semantics match the storage/filesystem architecture instead of exposing raw partition enumeration as the contents of Local Disk (C:).
- Observed AH532 behavior before this correction: Explorer opened C: as a seven-partition physical-disk listing; individual partitions did not open; clicks in the Explorer pane caused visible flicker; sidebar items were not reliably actionable.
- Root architectural correction: C: must represent a logical mounted filesystem volume. The physical partition table is a separate low-level/diagnostic view and must not be presented as the normal contents of C:.
- Source change in kernel/src/files_mgr.rs: Computer -> C: now requests the NTFS mount path and opens the NTFS browser instead of the physical partition view. C: is labelled as a read-only NTFS system volume; AetherFS is labelled as a read/write RAM volume. Inert sidebar/toolbar clicks are no longer supposed to force a redraw.
- Commit: 1952605f58d137349428b71318072aad63291e17 (Fix Explorer volume semantics and suppress redraw on inert clicks).
- Build #166 (run 35931819785) previously completed successfully before this correction. Build #167 (run 35932421767) was triggered by the correction and must be verified before an ISO is presented as the current test artifact.
- Acceptance criteria for this task: (1) empty/inert clicks do not visibly flicker; (2) Explorer sidebar/input hit-testing is functional; (3) double-clicking C: opens the actual root contents of the selected NTFS system volume, not the partition table; (4) physical partitions remain available only through an explicitly separate diagnostic/storage view; (5) no host-disk writes are introduced.
- Important unresolved technical point: fs_ntfs::mount_first() must not be assumed to identify the user's intended C: volume when multiple NTFS/ExFAT partitions exist. The next source inspection must establish deterministic logical-volume selection from actual partition metadata, and if the NTFS root parser returns empty data, fix the filesystem path rather than masking it in the Explorer UI.
- Working rule added: do not call the Explorer complete until source inspection, CI/QEMU verification, and the required AH532 runtime test support these acceptance criteria.

### 2026-09-24 — Repeated command-routing mistake: GUI terminal vs shell.rs
- Error recorded as a project-level process failure: the `77` diagnostic was first modified in `kernel/src/shell.rs`, even though the command was being entered in the desktop GUI terminal.
- The desktop GUI terminal has its own command dispatcher in `kernel/src/desktop.rs`, specifically `run_cmd()`, plus its own terminal buffer (`TERM_ROWS=16`, `TERM_COLS=52`). Changes to `shell.rs` therefore do not change the GUI terminal command path unless source inspection proves that routing.
- This mistake was repeated after it had already been identified once. It must not be repeated again.
- **Mandatory project rule:** when the user runs a command in the desktop GUI terminal, first inspect `kernel/src/desktop.rs` -> `run_cmd()` -> the exact command handler. Do not modify `kernel/src/shell.rs` unless the GUI routing to it is explicitly proven from source.
- The same rule applies to diagnostic commands such as `77` and `dsk`: verify the actual GUI dispatch path before changing command behavior.
- Terminal-capacity finding: the GUI terminal keeps only 16 visible rows and scrolls older lines out of its buffer; it also limits each line to 52 columns. Long diagnostics must therefore be designed/validated with this constraint in mind.



### 2026-09-28 — Native 1366x768 nature wallpaper graphics check
- User-requested graphics validation: replace the procedural desktop background with a photographic nature image at exactly 1366x768.
- Selected Wikimedia Commons image: “Comeragh Mountains Lake.jpg” by Mik Herman (Citarny), 1920x1080 source, licensed CC BY-SA 3.0.
- CI downloads the source during the kernel build and converts it to exact 1366x768 RGB565 (2,098,176 bytes), then bundles it into the native kernel image.
- Desktop uses the photographic wallpaper only at 1366x768; 800x600 retains the existing procedural fallback.
- The wallpaper is rendered through the existing framebuffer path; no Intel KMS/GGTT/GSM behavior is changed.
- Required validation: green GitHub Actions build, then boot the new ISO on AH532 at 1366x768 and visually inspect image sharpness, full-screen coverage, color conversion, and stability during window movement.


### 2026-09-28 — Wallpaper CI correction
- First wallpaper commit 900d147078ff215ddb42594550c111ea493d48ea downloaded the source and converted it successfully, but the Makefile size check was malformed by Make variable expansion (`test "" -eq 2098176`).
- No ISO was published from that failed run.
- Corrected only the shell/Makefile size validation; wallpaper source, rendering path, resolution, and graphics driver code are unchanged.
- Next required evidence: green build run and AH532 visual test at 1366x768.


### 2026-09-28 — Native cursor library and Settings → Mouse → Cursor
- Integrated Phinger Cursors by Philipp Schaffrath (CC BY-SA 4.0).
- CI downloads upstream SVG artwork, creates 24 Aether variants (4 cursor shapes × 6 colors), rasterizes to 16x16 RGBA, and bundles them.
- Settings → Mouse → Cursor now presents a real 24-item preview/selection grid with Apply and Cancel.
- Applying a selection changes the actual native desktop cursor renderer immediately.
- KMS/GGTT/GSM and framebuffer paths are unchanged.


### 2026-09-28 — Desktop click flicker: broken backbuffer experiment reverted
- Investigated the remaining global desktop flicker where interactive button clicks could flash the entire screen while inert/empty clicks no longer did.
- Source inspection showed the GUI is currently rendered directly into the visible framebuffer; there is no complete backbuffer/present pipeline. Existing Intel Gen6 vblank support is available, but that alone does not prove the flicker cause.
- A software-backbuffer experiment was introduced in commit ed04e9d60e4fdfd0c6a74fb85cb8b0170bba740e. The experiment was incomplete because the rendering path was not coherently switched to render a complete frame and present it; on AH532 it broke desktop startup and showed the old initial desktop with colored lines/cubes.
- The experiment is classified as FAILED/REJECTED and must not be repeated in its incomplete form.
- Restored kernel/src/graphics.rs to the known-good content from 386110499f45fa4c8fc9aba8241c3adb206a5d06.
- Revert commit: 28b4f0882a1c849544067abe98908fabdb8209c8; merged as 90196ab0daf5ef826ca9f63f23bd30758f3e9881.
- No ISO is considered testable from this change until GitHub Actions verifies the merged state.
- Next graphics investigation: inspect framebuffer allocation/mapping and boot-time memory setup, then design a complete rendering-target/present architecture rather than another partial backbuffer patch. Avoid static 8 MiB .bss framebuffer experiments.


### 2026-09-29 — Documentation truth audit
- Audited the repository markdown tree against the current source tree and latest recorded hardware evidence.
- Corrected stale source-snapshot, build, Wi-Fi, Media Player, video-status, native-app-contract, audit-baseline, vision and roadmap wording.
- Historical audit material is retained, but current implementation claims are routed through docs/STATUS.md and the current source tree.
- CI for documentation commit 0bb996ed5ab809ae6c73709262ea964e7c3f3224 includes Aether OS build #699, which was in progress at audit time.


### 2026-09-29 — Desktop restoration after Linux runtime integration
- User reported that build #705 still boots the starter/test screen and does not reach the native Aether Desktop.
- Source audit of commit 7aa10aacbfcad5adaeb73545d1807a87802cb077 found that the earlier fix only removed `mod runtime;`; it did not remove the boot-time `start_capability_ring3_test()` foreground transition.
- `kernel_main()` therefore still entered the Ring3 capability test before the desktop block. The desktop was only reachable after that path returned, which is incompatible with the required architecture: Aether Desktop must remain the host and compatibility personalities must launch inside it.
- Corrective change: defer the Ring3 capability regression during boot and continue directly to the existing native Desktop initialization. The test remains available as an explicit test path rather than a mandatory boot stage.
- Required validation: one green GitHub Actions build, then AH532 boot verification that the normal Aether Desktop appears before any Linux-personality work is resumed.


### 2026-09-29 — Root cause found: Desktop rendered into hidden backbuffer
- After the previous boot-flow fixes, source audit showed the Desktop path was actually reached, but `graphics::init()` enabled the 8 MiB software backbuffer whenever the 32bpp framebuffer fit.
- `graphics::put_pixel()` then rendered the Desktop into `BACKBUFFER`, while the visible Multiboot LFB was not updated by the Desktop render loop. This left the visible screen showing the earlier framebuffer/test contents, making it appear that Desktop never started.
- Corrective change: restore the known-good direct-LFB rendering path by disabling the incomplete backbuffer experiment at initialization. The backbuffer storage and API remain for future complete present-pipeline work, but it is no longer selected by default.
- This is the first fix in this sequence that addresses the actual rendering path rather than merely changing boot-stage ordering.


### 2026-09-29 — First Windows EXE desktop target
- Architecture goal changed: Aether will directly launch applications from Windows, Linux, and Android ecosystems; the previous Personality-based model is no longer the target architecture.
- Added the first safe Windows target: `/hello.exe`, a real PE32+ AMD64-formatted test image installed during storage initialization.
- Added a desktop `Hello.exe` icon and double-click path into the PE compatibility layer.
- Safety invariant: boot and native Desktop paths are unchanged; the PE milestone validates MZ/PE64 headers and opens an Aether window, but deliberately does not execute foreign machine code yet.
- Next milestone: connect validated PE images to the already-isolated Ring3 process path, then implement the first minimal Win32 API needed by a real external Windows program.


### 2026-09-29 — Real Winamp target
- The Windows target now downloads the real official Winamp desktop installer during CI and extracts the original `winamp.exe`; Aether does not recreate or rewrite the player.
- Added a desktop `Winamp.exe` icon and a safe PE32/x86 recognition window.
- Winamp 5.9.x is a 32-bit Windows application, so this target deliberately exposes the next required runtime boundary: x86 PE + Win32 API execution on x86_64 Aether.
- Safety invariant: the existing boot/framebuffer/Desktop path remains unchanged; this milestone only packages and recognizes the real third-party executable.
