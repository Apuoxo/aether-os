# AH532 schematic provenance and extraction audit

Last reviewed: 2026-09-23

## 1. Verified target

The correct schematic family for the target machine is independently identified as:

- Fujitsu LIFEBOOK AH532 / A532
- Quanta FH6 / FH6C
- motherboard marking DA0FH6MB6E0
- Rev E
- schematic filename: `Fujitsu_FH6C_FH6_hm70_r0c_mb_0522.pdf`

Independent repair-documentation sources associate this exact schematic family with AH532/FH6 and DA0FH6MB6E0. The sources also report a 45-page schematic variant.

## 2. The PDF currently in our repository

The PDF extracted from our uploaded RAR is **not the verified FH6/HM70 document**.

Repository file:

`docs/AH532-SERVICE-MANUAL/DA0FH6MB6E0 rev E PDF .pdf`

Observed properties:

- 34 pages
- 663333 bytes
- metadata title: `FH2_MB_A_1221_1(FINAL)`
- extracted text contains Arrandale / HM55 / IBEX PEAK-M terminology

This is inconsistent with the independently identified AH532/FH6 target. It remains classified:

**REFERENCE-MISMATCH**

The existing technical index is therefore only an index of that mismatched PDF and must not be treated as an AH532 signal database.

## 3. Important correction

The filename `DA0FH6MB6E0 rev E PDF` was not enough to establish that the PDF content itself was the AH532 schematic.

The stronger identification is:

**AH532 target → Quanta FH6/FH6C → DA0FH6MB6E0 Rev E → FH6C/FH6 HM70 r0c_mb_0522 schematic family.**

The repository's current 34-page PDF does not match that chain internally.

## 4. Next required artifact

The next documentation artifact should be the actual **FH6/FH6C HM70 r0c_mb_0522** schematic, preferably the 45-page variant, with internal project/board identifiers verified before ingestion.

Verification checklist:

1. project name FH6/FH6C;
2. DA0FH6MB6E0 / related FH6 board identifier;
3. HM70 platform terminology;
4. internal schematic page count/structure;
5. display/GPU topology;
6. USB topology;
7. SATA/AHCI;
8. LAN/WLAN;
9. audio;
10. power/reset/EC/GPIO.

Only after these checks pass should signal-level data be promoted into the AH532 hardware profile.

## 5. Driver-development rule

Until the matching schematic is obtained:

- actual AH532 runtime observations remain authoritative;
- Intel Sandy Bridge/HD Graphics 3000 documentation remains authoritative for GPU architecture;
- the current mismatched PDF remains reference-only;
- no AH532 GMBUS/EDID, GPIO, power-sequencing or display-routing claim may be based solely on the 34-page PDF.

This protects the working video driver from a schematic provenance error.
