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

## 5. DA0FH6MB6E0 Rev E archive — provenance correction

The repository contains an archive named `DA0FH6MB6E0 rev E PDF .rar`. The **board identifier itself is strongly associated with Fujitsu LIFEBOOK AH532 / Quanta FH6** by multiple independent repair-documentation sources. citeturn0search0turn0search3turn0search6

However, the PDF actually extracted from our archive must **not** currently be treated as the AH532 schematic.

The extracted PDF has its own internal metadata/title and text that identify a different platform/project (including FH2 / Arrandale / HM55 / Ibex Peak-M terminology). That conflicts with the AH532 target. Therefore the repository now records two separate facts:

- **BOARD-ID ASSOCIATION:** `DA0FH6MB6E0 Rev E` ↔ AH532/FH6 — externally corroborated. citeturn0search0turn0search6
- **EXTRACTED-PDF CONTENT:** the specific 663333-byte, 34-page PDF currently inside our RAR does not provide reliable AH532-specific electrical evidence and is marked **REFERENCE-MISMATCH** until the correct FH6/HM70 schematic is obtained and verified.

This distinction is intentional. The filename/board identifier alone is not sufficient to promote the PDF's signal names, power rails, display routing, GPIO ownership, or controller details into AH532 hardware facts.

### 5.1 What can still be retained

The extracted document remains useful as a **separate historical/reference artifact** and its text can be searched for generic board-design concepts. It must not be used as the source of truth for AH532 driver development.

### 5.2 Correct target for future schematic work

The externally corroborated target is the **Fujitsu FH6/FH6C / DA0FH6MB6E0 Rev E** schematic family associated with AH532/A532. A separate source describes the same board family as Quanta FH6/FH6C and DA0FH6MB6E0. citeturn0search3turn0search6

Until the actual matching schematic is verified by its internal board/project identifiers, Aether driver work must continue to rely on:
1. real-AH532 runtime evidence;
2. Intel chipset/GPU documentation;
3. verified AH532/FH6 documentation;
4. only then, schematic-specific signal tracing.

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
