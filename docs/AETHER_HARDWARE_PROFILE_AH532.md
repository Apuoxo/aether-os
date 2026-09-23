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
| REFERENCE-MISMATCH | Document was inspected and found to describe another platform. |

## 1. Platform facts relevant to current development

| Item | Current value | Confidence |
|---|---|---|
| Machine | Fujitsu LIFEBOOK AH532 | AETHER / DOC |
| Display panel | 1366x768 class panel | DOC / AETHER context |
| PCH family | Intel HM76 / Panther Point | DOC / INFERRED |
| Integrated GPU actually detected by Aether | Intel HD Graphics 3000 | AETHER |
| GPU PCI ID | 8086:0116 | AETHER |
| GPU generation | Intel Sandy Bridge Gen6 | AETHER |
| Multiboot framebuffer | Working | AETHER |
| Stable tested desktop mode | 800x600 | AETHER |
| KMS/modeset path | Experimentally working | AETHER |
| Pipe/vblank diagnostics | Working | AETHER |
| GPU MMIO path | Usable | AETHER |
| GGTT/GSM mapping around 0xDF800000 | Unsafe; caused reboot | AETHER |

The 8086:0116 runtime result is authoritative for the GPU identity of the tested machine. Do not replace it with the older generic AH532 HD3000/HD4000 description.

## 2. Graphics architecture used by Aether

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

## 3. Known-safe and known-unsafe graphics operations

### Known-safe in the current tested path

- PCI configuration reads.
- BAR decoding.
- Guarded BAR0 MMIO mapping of the required register window.
- Read-only display-register snapshots.
- Read-only pipe/vblank diagnostics.
- Reading existing scanout state.
- Software drawing into the known-good framebuffer.

### Known-unsafe on this AH532

Mapping/accessing the GGTT/GSM physical region around 0xDF800000 previously caused the notebook to reboot.

Therefore:

- do not re-enable GGTT/GSM mapping casually;
- do not treat successful QEMU access as proof that the AH532 mapping is safe;
- any future attempt needs an isolated recovery-safe experiment and explicit new evidence.

## 4. EDID / connector state

The current video driver contains an explicit Sandy Bridge GMBUS/EDID path.

Current implementation includes:

- GMBUS register access;
- explicit EDID transaction;
- GMBUS timeout handling;
- SATOER checking;
- 128-byte EDID header/checksum validation;
- detailed timing descriptor #1 parsing;
- preferred-mode storage;
- framebuffer geometry matching.

This code is not yet equivalent to runtime proof of EDID success on the physical AH532. A dedicated diagnostic run is still required.

## 5. Service-manual correction

The repository contains:

DA0FH6MB6E0 rev E PDF .rar

It was extracted and inspected through GitHub Actions.

The extracted schematic is explicitly:

- Intel Calpella;
- Arrandale UMA;
- Arrandale 35W CPU;
- HM55 PCH (82801IBM);
- project FH2;
- dated 2009.

Therefore it is **REFERENCE-MISMATCH** for the current AH532 target.

It must not be used to infer AH532:

- Intel HD Graphics 3000 register layout;
- Panther Point/HM76 power sequencing;
- LVDS/eDP/HDMI routing;
- GPIO ownership;
- PCI resources;
- graphics power rails.

It is retained in the repository as a historical/reference artifact only.

## 6. Storage notes

The platform is expected to use the PCH SATA controller in AHCI mode. Current Aether contains read-only storage diagnostics and AHCI probing infrastructure, but physical-disk write operations remain prohibited.

The current hardware profile should not claim verified host-disk access unless a real-AH532 runtime result proves it.

## 7. Driver validation discipline

For AH532 hardware work:

1. Source change.
2. Build/CI result.
3. QEMU evidence where applicable.
4. Real-AH532 evidence for hardware-dependent behaviour.
5. Record failures and preserve known-good state.
6. Do not promote an inference to a hardware fact.

## 8. Next graphics test

The next controlled test is read-only EDID/GMBUS validation.

It must not:

- map GGTT/GSM;
- change display mode automatically;
- replace the stable framebuffer;
- perform speculative power sequencing.

Detailed graphics state is maintained in docs/VIDEO_DRIVER_STATUS.md.
