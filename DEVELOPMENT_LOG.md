# Aether OS Development Log

This file is the persistent engineering log for the Aether OS repository.

## Rules

- Do not claim a subsystem is complete without runtime evidence.
- Record source changes, build results, hardware results, failures, and the next controlled step.
- Preserve known-good states and avoid repeating experiments already shown unsafe.
- AH532 is a real-hardware validation target; QEMU success is not equivalent to AH532 success.
- Current graphics hardware: Intel Sandy Bridge HD Graphics 3000, PCI 8086:0116, Gen6.
- GGTT/GSM physical region 0xDF800000 is currently considered unsafe on AH532 after mapping attempts caused reboot. Do not re-enable such mapping without a new, justified test plan.

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
- GNU GRUB documentation confirms `terminal_output console` is a valid native console output and that `grub-mkrescue --install-modules=...` can restrict installed modules and their dependencies. 
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

### 2026-09-28 — Explorer MP3 double-click autoplay
- Control baseline: build #672, commit 386110499f45fa4c8fc9aba8241c3adb206a5d06.
- Explorer at this baseline already detects .mp3 files and routes them to desktop::open_media_path(), which opens the file and adds it to the Media Player playlist.
- The missing step was playback start: open_media_path() did not call media_player::play().
- Controlled change: call the existing media_player::play() after a successful open and playlist insertion. No decoder, HDA, storage, or graphics code changed.
- Acceptance: double-clicking an MP3 in AetherFS Explorer brings the Media Player forward and immediately starts playback through the existing native MP3/HDA path.

### 2026-09-28 — CI trigger retry for Explorer MP3 autoplay
- Source remains based directly on control baseline build #672.
- No runtime/source behavior changed; this log-only commit exists to retrigger the pull-request workflow for the already-reviewed MP3 autoplay change.
- The functional change remains exactly the prior commit: c025e53ac492ff6f3d92eaa9c1c5d22fe1a949ee.


### 2026-09-28 — Explorer HDD/NTFS MP3 autoplay
- Root cause: NTFS Explorer double-click stopped at `NTFS file preview not implemented`; AetherFS already used the media-player handoff.
- Added bounded NTFS file-data reads, an NTFS MP3 source in the native player, and NTFS `.mp3` double-click routing through the existing media-player handoff.
- No graphics, HDA, partition mounting, or decoder changes.


### 2026-09-28 — HDA playback cushion for Explorer UI load
- User observed a short digital `bebe`/glitch in ongoing MP3 playback when opening Explorer or clicking inside it.
- Source inspection found the HDA PCM ring is serviced from the desktop polling loop; the previous 8-period / 32 KiB ring could be exhausted during a synchronous Explorer redraw or filesystem operation.
- Increased the cyclic PCM DMA ring from 8 to 16 periods (64 KiB), with matching 16-entry BDL, CBL and LVI configuration.
- MP3 decoding, NTFS routing, Explorer behavior, graphics, and codec setup are unchanged.
- Acceptance: continuous MP3 playback while opening/clicking Explorer must remain clean.


### 2026-09-28 — NTFS Unicode filename preservation
- Source audit confirmed the NTFS directory parser was converting every non-ASCII UTF-16 filename code unit to '?' before Explorer received it.
- Replaced that lossy ASCII conversion with bounded UTF-8 encoding, including surrogate-pair handling, while keeping the existing fixed-size NtfsEntry ABI.
- Explorer, Media Player, filesystem reads, and HDA code were not changed in this step.
- Acceptance: Cyrillic and other Unicode NTFS filenames must reach the Explorer renderer intact instead of appearing as '?'.


### 2026-09-28 — Media/Explorer stabilization plan: RAM first, then audio/UI
- User test after the HDA buffer increase still reports a short CD-like playback stutter when actively navigating/clicking in Explorer. The previous harsher digital `bebe` artifact is reduced, but the interruption remains.
- Before changing the audio architecture, the next controlled step is RAM/memory diagnostics: establish what physical memory Aether detects, what portion is usable by the kernel, and current heap/allocator usage.
- Planned `MEM` diagnostic should report, within the existing GUI terminal's 16-row/52-column constraints where applicable: total RAM, usable RAM, kernel/heap totals and usage, free memory, and relevant DMA/PCM allocations.
- Do not assume the machine's physical 16 GiB is fully available to Aether until runtime evidence confirms the memory map and allocator state.
- After RAM evidence, planned fixes remain:
  1. diagnose and fix the `Artist - Song.mp3` NTFS Explorer open failure (with spaces around the hyphen);
  2. show current playing filename in Media Player;
  3. add real playback elapsed/total timing;
  4. add a visible playback progress indicator;
  5. decouple audio refill/service from the desktop/Explorer polling path so filesystem/UI activity cannot starve PCM playback;
  6. replace the temporary Explorer→Media Player synthetic-path coupling with a proper Open/FileObject/application-association layer.
- Rule for this sequence: one focused commit at a time, green CI before AH532 testing, and no speculative audio/GUI refactor before the RAM evidence is collected.


### 2026-09-28 — Multiboot2 RAM discovery / RAM command
- Added Multiboot2 memory-map discovery and the RAM terminal diagnostic.
- PMM remains unchanged at 64 MiB until runtime RAM output is verified on real hardware.
