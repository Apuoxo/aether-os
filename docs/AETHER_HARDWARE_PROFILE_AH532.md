# AETHER_HARDWARE_PROFILE_AH532

Reference hardware target for Aether OS.
Model: Fujitsu LIFEBOOK AH532.

Last reviewed: 2026-09-23.

## Confidence legend

| Tag | Meaning |
|---|---|
| DOC | Confirmed by a genuinely matching vendor/platform document or authoritative hardware database. |
| AETHER | Observed by Aether runtime on the user's actual AH532. |
| INFERRED | Strong chipset/device-family inference, not directly measured. |
| UNKNOWN | Not established. |
| REFERENCE-MISMATCH | Document was inspected and found to describe another platform/variant and must not be used as exact-machine evidence. |

## 1. Platform facts relevant to current development

| Item | Current value | Confidence |
|---|---|---|
| Machine | Fujitsu LIFEBOOK AH532 | AETHER / DOC |
| Display panel | 1366x768 class panel | DOC / AETHER context |
| PCH family | Panther Point / HM70-family board documentation; exact runtime PCH identity still to be separately established | DOC / INFERRED |
| Integrated GPU actually detected by Aether | Intel HD Graphics 3000 | AETHER |
| GPU PCI ID | 8086:0116 | AETHER |
| GPU generation | Intel Sandy Bridge Gen6 | AETHER |
| Multiboot framebuffer | Working | AETHER |
| Stable tested desktop mode | 800x600 | AETHER |
| KMS/modeset path | Experimentally working | AETHER |
| Pipe/vblank diagnostics | Working | AETHER |
| GPU MMIO path | Usable | AETHER |
| GGTT/GSM mapping around 0xDF800000 | Unsafe; caused reboot | AETHER |

The 8086:0116 runtime result is authoritative for the GPU identity of the tested machine.

## 2. Verified FH6/FH6C schematic

The repository now contains the cryptographically verified 45-page FH6/FH6C HM70 schematic:

- `Fujitsu_FH6C_FH6_hm70_r0c_mb_0522 SKCLAPPY.IN.pdf`
- 1,498,316 bytes
- SHA-256 `8011b1775a403b5d62ff816c278412f62c2e3d7873b955f2721fa664528badbc`
- 45 pages
- Project family FH6C_HM70 / DA0FH6MB6E0 Rev E.

Qualification: the document is a verified board-family/variant schematic, not proof of the exact CPU/GPU configuration installed in the tested AH532. It contains Ivy Bridge and optional N13P discrete-GPU sections, while the tested runtime GPU is Sandy Bridge HD3000 8086:0116.

## 3. Display routing documented by the schematic

### Internal LCD
Documented PCH-side nets include:
- LCD_EDIDCLK / LCD_EDIDDATA
- LCD_TXLCLKOUT+/-
- LCD_TXLOUT0+/-, 1+/-, 2+/-
- LVDS_DIGON
- LVDS_PWM
- LCD_BLON_I

LCD connector: CN33.
Panel power/backlight circuitry includes LCDVCC, LCD_BK_POWER and a dedicated LCD power switch.

### HDMI
Documented DDI-B path includes:
- DDPB_AUXN/P
- DDPB_HPD
- DDPB_0N/P .. DDPB_3N/P
- INT_HDMI_SCL / INT_HDMI_SDA

HDMI connector: CN6.
The schematic also shows DDC level shifting, DDC5V and HPD circuitry.

### Other display transports
DDPC/DDPD and FDI signals are documented. Their exact runtime use remains UNKNOWN until correlated with actual hardware/connector detection.

A dedicated signal-level reference is maintained in `docs/AH532-SCHEMATIC_DISPLAY_SIGNAL_MAP.md`.

## 4. Graphics architecture used by Aether

Current driver layers:
1. Multiboot framebuffer discovery.
2. Software drawing/compositor path.
3. Intel PCI display-controller discovery.
4. BAR0 register aperture discovery.
5. Guarded MMIO mapping.
6. Read-only Gen6 display-engine snapshot.
7. Existing primary-plane/scanout validation.
8. Explicit GMBUS/EDID path.
9. Future native modeset/graphics-memory stages.

The current design intentionally keeps the existing Multiboot framebuffer as the presentation surface while hardware access is being validated.

## 5. Known-safe and known-unsafe graphics operations

### Known-safe
- PCI configuration reads.
- BAR decoding.
- Guarded BAR0 MMIO mapping.
- Read-only display-register snapshots.
- Read-only pipe/vblank diagnostics.
- Reading existing scanout state.
- Software drawing into the known-good framebuffer.

### Known-unsafe on this AH532
Mapping/accessing the GGTT/GSM physical region around 0xDF800000 previously caused the notebook to reboot.

Therefore it remains disabled.

## 6. EDID / connector state

The current video driver contains an explicit Sandy Bridge GMBUS/EDID path:
- GMBUS register access;
- explicit EDID transaction;
- timeout handling;
- SATOER checking;
- 128-byte EDID header/checksum validation;
- detailed timing descriptor #1 parsing;
- preferred-mode storage;
- framebuffer geometry matching.

The verified schematic adds physical-board context for LCD EDID and HDMI DDC/HPD, but it does not establish the exact GMBUS-pin-to-connector mapping on the tested runtime configuration.

## 7. Schematic provenance correction

The older repository PDF extracted from `DA0FH6MB6E0 rev E PDF .rar` is a 34-page FH2/Arrandale/HM55 document and remains REFERENCE-MISMATCH.

It must not be used as AH532 electrical evidence.

The newly verified FH6/FH6C HM70 PDF is the board-family reference for physical signal tracing, with the variant qualification above.

## 8. Driver validation discipline

For AH532 hardware work:
1. Source change.
2. Build/CI result.
3. QEMU evidence where applicable.
4. Real-AH532 evidence for hardware-dependent behaviour.
5. Record failures and preserve known-good state.
6. Do not promote an inference to a hardware fact.

## 9. Next graphics test

The next controlled test is read-only EDID/GMBUS validation.

It must not:
- map GGTT/GSM;
- change display mode automatically;
- replace the stable framebuffer;
- perform speculative power sequencing.
