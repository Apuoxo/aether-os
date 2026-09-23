# Aether Laboratory Status

## Status basis

This file is a laboratory status document, not a release-version identifier. Historical version strings in the repository must not override source/runtime evidence.

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
10. A verified 45-page FH6/FH6C HM70 schematic has been added and read page-by-page.
11. Physical LCD/LVDS and HDMI/DDI-B display-routing information is now documented.

Not complete:

- full cold-boot native Intel modeset independent of firmware-provided scanout;
- safe GGTT/GSM initialization;
- hardware-accelerated rendering;
- complete connector/power-management handling;
- verified real-AH532 EDID acquisition;
- complete native Intel display driver.

The GGTT/GSM region around physical 0xDF800000 remains blocked because previous mapping attempts rebooted the AH532.

## Verified schematic status

The repository now contains:

- `Fujitsu_FH6C_FH6_hm70_r0c_mb_0522 SKCLAPPY.IN.pdf`
- 1,498,316 bytes
- SHA-256 `8011b1775a403b5d62ff816c278412f62c2e3d7873b955f2721fa664528badbc`
- 45 pages.

The document is a verified FH6/FH6C HM70-family board/variant schematic. It explicitly contains Ivy Bridge and optional N13P discrete-GPU variants, so it is used for physical board-family signal routing and not as proof of the exact installed GPU.

The actual tested machine remains authoritative: Sandy Bridge HD Graphics 3000, 8086:0116.

Detailed display routing: `docs/AH532-SCHEMATIC_DISPLAY_SIGNAL_MAP.md`.

## Previous schematic correction

The older PDF extracted from `DA0FH6MB6E0 rev E PDF .rar` is a 34-page FH2/Arrandale/HM55 document and remains REFERENCE-MISMATCH. It must not be used as AH532 electrical evidence.

## Next controlled stage

The next graphics stage is explicit read-only EDID/GMBUS validation on QEMU where applicable and then real AH532.

The test must preserve the stable framebuffer and must not:

- map GGTT/GSM;
- automatically change display mode;
- replace framebuffer ownership;
- perform speculative power sequencing.

Detailed status: `docs/VIDEO_DRIVER_STATUS.md`.
