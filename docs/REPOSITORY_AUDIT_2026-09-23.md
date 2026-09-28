# Repository and Hardware Archive Audit

Audit date: 2026-09-23
Repository: Apuoxo/aether-os
Purpose: establish which uploaded materials are authoritative for current Aether development and prevent stale archives/documentation from being mistaken for the current implementation.

## 1. Materials examined

### Source archive

aether-os-sources.zip
- Repository size: 2,171,594 bytes.
- Successfully unpacked by GitHub Actions.
- Unpacked source snapshot contains 87 files, including source, documentation and prebuilt diagnostic/build artifacts.
- The archive timestamps are mostly 2026-09-17 through 2026-09-21.
- This is a historical source snapshot, not the current development tree.

### AH532-labelled service-manual archive

DA0FH6MB6E0 rev E PDF .rar
- GitHub Actions extraction succeeded.
- The extracted PDF is 663,333 bytes and 34 pages.
- The archive also contains two 640x640 JPEGs and a text file with source links.

## 2. Critical finding: the service manual is NOT an AH532 hardware schematic

The extracted PDF identifies itself as Intel Calpella / Intel Calpella Arrandale UMA. It contains Arrandale 35W, PCH 82801IBM (HM55), project FH2, and a block diagram dated 21 December 2009.

This is not the Sandy Bridge / Panther Point HM76 platform used by the current AH532 runtime target.

Therefore this PDF must NOT be used as an authoritative register, pinout, power-sequencing, or graphics schematic source for the AH532 HD Graphics 3000 driver.

It is classified as REFERENCE-MISMATCH. It may describe a related Fujitsu/Quanta-era platform, but it is not AH532 evidence.

## 3. Source archive versus current tree

The archived kernel/src/drivers/video.rs is about 1.2 KiB and is only a software-framebuffer wrapper.

The current kernel/src/drivers/video.rs is substantially newer and contains:
- Intel PCI discovery.
- HD Graphics 3000 identification: PCI 8086:0116.
- BAR decoding.
- Guarded BAR0 MMIO mapping.
- Sandy Bridge display-register snapshots.
- Existing scanout detection.
- GMBUS register snapshot.
- Explicit EDID read path.
- EDID validation and preferred-mode parsing.
- Existing-mode / Multiboot framebuffer matching.
- Driver diagnostic marker.
- Normal-boot protection against display-register and GGTT/GSM mutation.

The archive must therefore never be copied over the current kernel tree wholesale.

## 4. Documentation staleness found

Several historical documents still describe Aether as version 0.4, a graphics skeleton, or a pre-GUI foundation.
The current source also contains stale version strings in different places. These labels are not reliable release identifiers.

For engineering decisions, use this authority order:
1. Current source tree and commit.
2. QEMU runtime evidence.
3. Real AH532 runtime evidence.
4. DEVELOPMENT_LOG.md.
5. Historical archives and old version labels.

## 5. Current graphics evidence

The current development log records:
- Intel Sandy Bridge HD Graphics 3000.
- PCI 8086:0116.
- Gen6 display engine.
- KMS/modeset experimentally working on AH532.
- Stable 800x600 desktop path.
- Dynamic framebuffer geometry fixed.
- MMIO/KMS path usable.
- GGTT/GSM mapping around physical 0xDF800000 caused an AH532 reboot and remains disabled.
- Next direction: read-only scanout/EDID/GMBUS validation.

These are engineering-state statements, not a claim that a complete Intel graphics driver exists.

## 6. Documentation policy

AH532 hardware facts must be tagged DOC, AETHER, INFERRED, UNKNOWN, or REFERENCE-MISMATCH.
The extracted FH2/HM55/Arrandale PDF is REFERENCE-MISMATCH for AH532.

## 7. Consequence for video-driver development

The uploaded service manual does not give us a valid AH532/HM76 schematic for HD Graphics 3000.
Graphics work must continue from the actual 8086:0116 hardware, the current Gen6 video driver, observed MMIO/KMS/pipe behaviour, safe read-only diagnostics, and a genuinely matching AH532/HM76 or Intel Sandy Bridge source when obtained.

The known-unsafe GGTT/GSM mapping must remain disabled until new evidence justifies a separate recovery-safe experiment.

## 8. Conclusion

The repository now distinguishes historical source, current implementation, and hardware-reference material.
The most important correction is that DA0FH6MB6E0 rev E is an Arrandale/HM55/FH2 schematic, not an AH532/HM76 schematic.