# AH532 schematic provenance and extraction audit

Last reviewed: 2026-09-23

## Purpose

This document separates the identity of the motherboard from the identity of the PDF currently stored in the repository. This prevents a misleading filename from becoming a hardware fact.

## 1. Board identity

The motherboard identifier **DA0FH6MB6E0 Rev E** is independently associated with the Fujitsu LIFEBOOK AH532 / Quanta FH6 platform by multiple repair-documentation sources. These sources also associate the FH6/FH6C schematic family with AH532/A532. 

- RepairLap explicitly labels DA0FH6MB6E0 Rev E as Fujitsu LIFEBOOK AH532 / Quanta FH6.
- RealSchematic associates AH532/A532 with Quanta FH6/FH6C and DA0FH6MB6E0.
- Vinafix identifies the FH6/FH6C schematic family and DA0FH6MB6E0 Rev E with Fujitsu AH532.

## 2. What is actually inside our archive

The RAR was extracted by GitHub Actions.

Current extracted PDF:

- filename: `DA0FH6MB6E0 rev E PDF .pdf`
- size: 663333 bytes
- pages: 34
- searchable text: extracted successfully
- metadata title: `FH2_MB_A_1221_1(FINAL)`
- extracted text contains platform terminology including **Arrandale**, **HM55**, and **IBEX PEAK-M**.

Those internal identifiers conflict with the AH532/FH6 target. Consequently the PDF is currently classified:

**REFERENCE-MISMATCH — DO NOT USE AS AH532 SCHEMATIC SOURCE OF TRUTH.**

## 3. Why the distinction matters

The extracted text contains apparently real electrical-design information such as LVDS, DDI, FDI, SATA, USB, PCH and power-rail names. Those details may be useful as generic historical reference, but they cannot safely be mapped onto the user's AH532.

In particular, do not derive from this PDF:

- Sandy Bridge HD Graphics 3000 register assumptions;
- HM76/Panther Point signal routing;
- AH532 LVDS/eDP/HDMI wiring;
- AH532 GMBUS/EDID routing;
- GPIO ownership;
- USB controller wiring;
- exact power/reset sequencing;
- exact component designators on the AH532 board.

## 4. Repository extraction products

The repository retains:

- original RAR;
- extracted PDF;
- `manual-text.txt`;
- `first-pages.txt`;
- `pdfinfo.txt`;
- `manual-identity-report.md`;
- `schematic-pages.txt`;
- `schematic-technical-index.md`.

The technical index is explicitly a navigation aid for the extracted PDF, not proof of AH532 relevance.

## 5. Current Aether evidence hierarchy

For AH532 hardware development use this order:

1. **AETHER:** measurements and runtime diagnostics from the actual AH532.
2. **Verified AH532/FH6 documentation:** documents whose internal identifiers match the target board.
3. **Intel documentation:** GPU/PCH specifications and register documentation.
4. **Reference material:** generic or mismatched schematics, clearly marked as such.

No reference-mismatch document may override a real-AH532 observation.

## 6. Next required document

The next useful artifact is the **actual FH6/FH6C DA0FH6MB6E0 Rev E schematic** whose internal project/board identifiers match the AH532 target.

Once obtained, it should be compared against:

- board identifier;
- project name;
- chipset/PCH;
- CPU family;
- GPU/display topology;
- USB topology;
- LAN/WLAN controllers;
- audio codec;
- page count and schematic structure.

Only after that verification should its signal-level information be promoted into the AH532 hardware profile.

## 7. Driver impact

For the current graphics driver, this audit means:

- the real-AH532 PCI result `8086:0116` remains authoritative;
- the working KMS/MMIO/VBlank observations remain authoritative;
- the unsafe GGTT/GSM result remains authoritative;
- the extracted mismatched PDF must not be used to justify a new GMBUS/EDID or display-memory access path.

This deliberately prevents a documentation error from turning into a hardware-driver regression.
