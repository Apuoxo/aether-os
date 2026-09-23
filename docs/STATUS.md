# Aether Laboratory Status

## Status basis

This file is a laboratory status document, not a release-version identifier. Historical version strings in the repository are stale and must not override source/runtime evidence.

## Current engineering state — 2026-09-23

Aether currently has a working x86_64 native kernel/userspace foundation with:

- physical memory management;
- paging and separate user/kernel address spaces;
- Ring3 ELF execution path;
- AetherFS RAM storage path;
- read-only storage diagnostics and AHCI probing infrastructure;
- PS/2 input;
- Multiboot framebuffer graphics;
- XP-style desktop/windowing work;
- a staged Intel Sandy Bridge Gen6 video-driver layer;
- QEMU CI/smoke-test infrastructure.

## Graphics status

Target hardware: Fujitsu LIFEBOOK AH532 with Intel HD Graphics 3000, PCI 8086:0116, Gen6.

Confirmed/implemented stages:

1. Multiboot framebuffer is usable.
2. 800x600 desktop path is stable.
3. Framebuffer geometry is handled dynamically.
4. Intel display PCI function is discovered.
5. BAR0 MMIO access is guarded.
6. Gen6 display registers can be snapshotted read-only.
7. Existing hardware scanout can be compared against the Multiboot framebuffer.
8. GMBUS/EDID support exists as an explicit, non-automatic path.
9. Safe video diagnostic command vdiag exists.

Not complete:

- full cold-boot native Intel modeset independent of firmware-provided scanout;
- safe GGTT/GSM initialization;
- hardware-accelerated rendering;
- complete connector/power-management handling;
- verified real-AH532 EDID acquisition;
- complete native Intel display driver.

The GGTT/GSM region around physical 0xDF800000 remains blocked because previous mapping attempts rebooted the AH532.

Detailed status: docs/VIDEO_DRIVER_STATUS.md.

## Hardware documentation correction

The uploaded DA0FH6MB6E0 rev E service-manual archive has been extracted and inspected.

It is an Intel Calpella / Arrandale UMA, HM55, FH2 schematic, not an AH532/HM76 Sandy Bridge schematic.

It is therefore classified as REFERENCE-MISMATCH and must not be used as authoritative AH532 graphics documentation.

Detailed repository/material audit: docs/REPOSITORY_AUDIT_2026-09-23.md.

## Source archive correction

aether-os-sources.zip is a historical snapshot. It contains an early software-framebuffer video driver and stale project documentation. It must not overwrite the current kernel tree.

## Next controlled stage

The next graphics stage is explicit read-only EDID/GMBUS validation on QEMU and then real AH532. No GGTT/GSM mapping and no automatic mode change should be introduced as part of that test.
