# Intel Gen6 Video Driver Status

Last reviewed: 2026-09-23
Target: Fujitsu LIFEBOOK AH532, runtime-detected Intel Sandy Bridge HD Graphics 3000, PCI 8086:0116.

## 1. Evidence levels

### Confirmed on real AH532
- HD Graphics 3000 / PCI device 8086:0116.
- Sandy Bridge / Gen6 display engine.
- Existing 800x600 framebuffer path is stable.
- 1366x768 geometry handling was corrected by removing the hard-coded 799x599 mouse clamp.
- KMS/modeset path has been observed working.
- Pipe/vblank diagnostics have been observed.
- MMIO path is usable.
- Attempts to map/access the GGTT/GSM physical region around 0xDF800000 caused a reboot; that path remains disabled.

### Implemented in current source
- Intel PCI display-controller discovery.
- BAR0 decoding and guarded MMIO mapping.
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
- Full native Intel modesetting from cold boot without relying on firmware/Multiboot-provided scanout.
- Full framebuffer allocation in Intel graphics memory.
- Safe GGTT/GSM setup on this AH532.
- Hardware-accelerated rendering.
- Hardware cursor.
- Complete connector/port discovery.
- Reliable EDID acquisition on the real AH532.
- DP/HDMI/LVDS-specific policy and power sequencing.
- Suspend/resume graphics state management.

## 2. Verified AH532/FH6C schematic evidence

A correct 45-page FH6/FH6C HM70 schematic has now been obtained and cryptographically verified in the repository:

- File: `Fujitsu_FH6C_FH6_hm70_r0c_mb_0522 SKCLAPPY.IN.pdf`
- Size: 1,498,316 bytes.
- SHA-256: `8011b1775a403b5d62ff816c278412f62c2e3d7873b955f2721fa664528badbc`
- Page count: 45.
- Project family: FH6C_HM70 / Quanta FH6/FH6C / DA0FH6MB6E0 Rev E family.

The schematic has been read page-by-page. It is useful as a verified board-family and physical signal-routing source, but it must **not** be treated as proof of the exact CPU/GPU population of the tested AH532.

Important variant qualification:
- The verified schematic explicitly depicts an Ivy Bridge processor section, HM70 Panther Point, and optional N13P discrete-GPU variants.
- The tested AH532 runtime instead reports Sandy Bridge HD Graphics 3000, PCI 8086:0116, Gen6.
- Therefore Ivy Bridge eDP signals and N13P GPU blocks in the schematic are variant/documentation evidence, not runtime facts for the tested machine.

## 3. Display signal map established from the verified schematic

### Internal LCD / LVDS
The PCH display sheet documents:
- LCD_BLON_I
- LVDS_DIGON
- LVDS_PWM
- LCD_EDIDCLK
- LCD_EDIDDATA
- LCD_TXLCLKOUT+/-
- LCD_TXLOUT0+/-
- LCD_TXLOUT1+/-
- LCD_TXLOUT2+/-

The LCD connector is CN33. The schematic also shows LCDVCC, LCD_BK_POWER, LVDS_DIGON_R, and a panel power switch. EDID pull-ups are shown as R326/R327, 2.2K to 3V_S0.

### HDMI / DDI-B
The PCH sheet documents DDI-B auxiliary/HPD/TMDS routing:
- DDPB_AUXN/P
- DDPB_HPD
- DDPB_0N/P through DDPB_3N/P

These route to the HDMI connector CN6 through the documented TMDS/HPD/DDC circuitry. HDMI DDC is represented by HDMI_DDCCLK and HDM_DDCDATA, with DDC5V and HPD circuitry.

### Other DDI paths
The schematic also documents DDPC and DDPD auxiliary, HPD, data and control nets. Their exact use on the tested machine remains a board-variant question until runtime connector detection is established.

### FDI
The schematic documents PCH FDI links:
- FDI_TXP/N0..7
- FDI_RXP/N0..7
- FDI_FSYNC0/1
- FDI_LSYNC0/1
- FDI_INT

These describe the CPU↔PCH display transport in the documented platform variant. They are not a substitute for the Gen6 HD3000 register model.

## 4. Important separation: schematic vs runtime GPU

The verified schematic provides physical board routing and power/control clues. It does not override runtime PCI identification.

Authoritative runtime GPU fact:
- 8086:0116 = Sandy Bridge Intel HD Graphics 3000 / Gen6 on the tested AH532.

Do not map the schematic's Ivy Bridge CPU eDP block directly onto the runtime HD3000 without a register/platform cross-check.

## 5. Current driver architecture

Normal initialization deliberately follows a non-destructive path:
1. Find the Intel display PCI function.
2. Decode BARs without rewriting PCI configuration.
3. Select BAR0 as the register aperture and reject unexpected BAR types.
4. Map only the required MMIO window.
5. Read display registers.
6. Detect whether an already-programmed primary plane matches the Multiboot framebuffer.
7. Keep the existing software framebuffer as the presentation surface.

This is intentionally a staged driver, not yet a complete i915-class implementation.

## 6. EDID/GMBUS status

The current source contains an explicit read_edid() path and probe_edid() pin probing for Sandy Bridge GMBUS pins.
Normal boot does not automatically start the EDID transaction.

The verified schematic now gives physical-board context for EDID:
- internal LCD: LCD_EDIDCLK/LCD_EDIDDATA;
- HDMI path: HDMI_DDCCLK/HDM_DDCDATA and associated DDC/HPD circuitry.

This does **not** prove that a particular GMBUS pin maps to a particular physical connector on the tested machine. Runtime probing is still required.

## 7. Safety constraints

Do not re-enable access to the known-unsafe GGTT/GSM physical region 0xDF800000 merely because MMIO works.
Do not use the old 34-page FH2/HM55/Arrandale PDF as AH532 signal evidence.
Do not claim KMS=READY means the complete Intel graphics driver is finished.
Do not replace the stable framebuffer path until the replacement path has equivalent real-hardware evidence.

## 8. Next controlled graphics stage

Stage G6-EDID-1:
1. Keep current MMIO-only initialization unchanged.
2. Add/use a diagnostic-only command that invokes EDID probing explicitly.
3. Probe one Sandy Bridge GMBUS pin at a time.
4. Log controller status, timeout/SATOER state, byte count and validation result.
5. Correlate successful EDID with the physical signal map, without assuming connector identity from the schematic alone.
6. Do not change modes or framebuffer ownership.
7. Run in QEMU where applicable, then on AH532.
8. If AH532 EDID succeeds, record raw 128-byte block checksum and parsed timing, but do not automatically modeset from it.

Only after stable EDID/connector evidence should the project advance toward native display policy and graphics-memory stages.
