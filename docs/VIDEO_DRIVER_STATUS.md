# Intel Gen6 Video Driver Status

Last reviewed: 2026-09-23
Target: Fujitsu LIFEBOOK AH532, Intel Sandy Bridge HD Graphics 3000, PCI 8086:0116.

## 1. Evidence levels

### Confirmed on real AH532
- HD Graphics 3000 / PCI device 8086:0116.
- Gen6 display engine.
- Existing 800x600 framebuffer path is stable.
- 1366x768 geometry handling was corrected by removing the hard-coded 799x599 mouse clamp.
- KMS/modeset path has been observed working.
- Pipe/vblank diagnostics have been observed.
- MMIO path is usable.
- Attempts to map the GGTT/GSM physical region around 0xDF800000 caused a reboot; that path remains disabled.

### Implemented in current source
- PCI display-controller discovery.
- BAR0 decoding and guarded 1 MiB MMIO mapping.
- Read-only display-engine snapshot.
- PIPEA/PIPEB and primary-plane scanout inspection.
- Matching of the hardware scanout surface against the Multiboot framebuffer.
- GMBUS register snapshot.
- Explicit EDID transaction entry point.
- EDID header/checksum validation.
- First detailed timing descriptor parsing.
- Preferred-mode storage and framebuffer geometry validation.
- Safe video diagnostic shell command: vdiag.

### Not yet proven complete
- Full native Intel modesetting from a cold boot without relying on the firmware/Multiboot-provided scanout.
- Full framebuffer allocation in Intel graphics memory.
- Safe GGTT/GSM setup on this AH532.
- Hardware-accelerated rendering.
- Hardware cursor.
- Complete connector/port discovery.
- Reliable EDID acquisition on the real AH532.
- DP/HDMI/LVDS-specific policy and power sequencing.
- Suspend/resume graphics state management.

## 2. Current driver architecture

Normal initialization deliberately follows a non-destructive path:
1. Find the Intel display PCI function.
2. Decode BARs without rewriting PCI configuration.
3. Select BAR0 as the register aperture and reject unexpected BAR types.
4. Map only the required MMIO window.
5. Read display registers.
6. Detect whether an already-programmed primary plane matches the Multiboot framebuffer.
7. Keep the existing software framebuffer as the presentation surface.

This is intentionally a staged driver, not yet a complete i915-class implementation.

## 3. EDID/GMBUS status

The current source contains an explicit read_edid() path and probe_edid() pin probing for Sandy Bridge GMBUS pins.
Normal boot does not automatically start the EDID transaction.
The EDID path validates the standard 128-byte block header and checksum and parses detailed timing descriptor #1.

Important: source presence is not runtime proof. Real-AH532 EDID success still requires a hardware test and captured diagnostic output.

## 4. Safety constraints

Do not re-enable access to the known-unsafe GGTT/GSM physical region 0xDF800000 merely because MMIO works.
Do not use the uploaded DA0FH6MB6E0 rev E schematic as AH532 graphics hardware documentation: that document is an Arrandale/HM55/FH2 schematic.
Do not claim KMS=READY means the complete Intel graphics driver is finished.
Do not replace the stable framebuffer path until the replacement path has equivalent real-hardware evidence.

## 5. Next controlled graphics stage

Stage G6-EDID-1:
1. Keep the current MMIO-only initialization unchanged.
2. Add a diagnostic-only command that invokes EDID probing explicitly.
3. Probe one GMBUS pin at a time and log controller status, timeout, SATOER, byte count and validation result.
4. Do not change modes or framebuffer ownership.
5. Run first in QEMU where the path is available; then on AH532.
6. If EDID succeeds on AH532, record the raw 128-byte block checksum and parsed timing, but do not automatically modeset from it.

Only after stable EDID/connector evidence should the project consider the next display-engine stage.

## 6. Historical source comparison

The uploaded source archive contains an early video.rs that only wraps the existing graphics framebuffer.
The current driver is therefore a later development layer and should be treated as the authoritative implementation for the current video work.