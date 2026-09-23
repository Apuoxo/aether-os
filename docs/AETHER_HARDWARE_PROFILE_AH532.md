# AETHER_HARDWARE_PROFILE_AH532

**Reference hardware target for Aether OS**  
**Model family:** Fujitsu LIFEBOOK AH532 (15.6")  
**Note:** Earlier conversation also referenced “Amilo AH530”; AH530 is a *different* generation (HM55/Arrandale). This profile is for **AH532**.

**Last updated:** 2026-09-21  
**Aether real-hardware status:** Partial (boot, desktop, PS/2, RAM AetherFS). **Host SSD access: NOT VERIFIED on AH532.**

---

## Confidence legend

| Tag | Meaning |
|-----|---------|
| **DOC** | Confirmed by vendor datasheet / review / PCI ID database |
| **AETHER** | Observed by Aether runtime on the user's machine |
| **INFERRED** | Strong inference from chipset generation; not measured on this unit |
| **UNKNOWN** | Not established |

---

## 1. Platform overview

| Item | Value | Confidence | Source |
|------|--------|------------|--------|
| Product | LIFEBOOK AH532 | DOC | Fujitsu datasheets, Notebookcheck |
| Form factor | Clamshell 15.6" | DOC | Fujitsu factsheet |
| Display | 1366×768 HD LED | DOC | Fujitsu datasheet |
| Chipset | **Intel HM76 Express (Panther Point)** | DOC | Fujitsu datasheet, Notebookcheck |
| CPU options | 2nd/3rd gen Core (Sandy/Ivy Bridge mobile) e.g. i5-3210M | DOC | Fujitsu datasheet |
| RAM | 2× SO-DIMM DDR3-1600, max 16 GB | DOC | Fujitsu datasheet |
| Storage interface | **SATA** (2.5") | DOC | Fujitsu datasheet |
| Factory storage | 1× 2.5" SATA HDD typical | DOC | Fujitsu / reviews |
| User config | **Two physical SSDs** | AETHER (user report) | User statement 2026-09-21 |
| Ethernet | Realtek RTL8111F Gigabit | DOC | Fujitsu datasheet |
| Wi-Fi | Intel Centrino Wireless-N 2230 (common SKU) | DOC | Fujitsu US datasheet |
| Audio codec | Realtek ALC269 (VC2) | DOC | Fujitsu / Notebookcheck |
| USB | 3× USB 3.0 + 1× USB 2.0 | DOC | Fujitsu datasheet |
| Graphics | Intel HD 3000/4000 ± optional NVIDIA Optimus | DOC | Fujitsu / Notebookcheck |

---

## 2. Chipset / PCH (storage-relevant)

### 2.1 Intel HM76 (Panther Point)

- **DOC:** HM76 integrates 6-port SATA controller supporting AHCI and IDE modes.
- **Typical PCI SATA function (AHCI mode):**
  - Vendor `8086`, Device **`1E03`** — “7 Series Chipset Family 6-port SATA Controller [AHCI mode]”
  - Class `01`, Subclass `06`, ProgIF `01` (AHCI)
  - Source: pci.ids / Linux ahci table (`0x1e03` Panther M AHCI)
- **IDE mode Device IDs (if BIOS set to IDE):**
  - `1E01` / `1E00` / `1E08` / `1E09` (7 Series IDE-mode variants) — **DOC** pci.ids
- **LPC (HM76):** Device ID often `1E59` (HM76 Express LPC) — **DOC** pci.ids

### 2.2 Why Aether ATA PIO fails on AH532

| Hypothesis | Likelihood | Notes |
|------------|------------|-------|
| Controller in **AHCI mode**; legacy ports 0x1F0 not connected | **HIGH** | Default for modern notebooks; matches “no device / floating” in Aether ATA probe on AH532 |
| BIOS SATA mode = AHCI only (no IDE/Compatibility) | **HIGH** | Common on HM76 notebooks |
| Wrong primary/secondary I/O mapping | MEDIUM | Only matters if IDE mode enabled |
| Disks on non-primary SATA port not visible to legacy primary | MEDIUM | AHCI uses port registers, not 0x1F0 |
| Need AHCI driver (ABAR MMIO) | **HIGH** | Required path for AHCI mode |

**AETHER observed:** My Computer shows `Local Disk (C:): NOT DETECTED`; only AetherFS RAM 32 KiB. Consistent with ATA PIO not seeing devices.

**Conclusion (diagnostic, not yet driver):** Do **not** assume ATA PIO will work on AH532. Next step is **PCI mass-storage dump + AHCI register dump (READ-ONLY)** on the real machine, then implement AHCI RO driver **only if** PCI shows AHCI (class 01.06 / DID 1E03 or similar).

---

## 3. BIOS / firmware

| Topic | Status |
|-------|--------|
| Brand / type | Phoenix SecureCore family common on Fujitsu of era; **AH532 exact BIOS vendor string: UNKNOWN** until read from real firmware |
| Legacy BIOS boot | Aether boots via Multiboot2/GRUB on AH532 — **AETHER** (user photos) |
| UEFI | Possible on some SKUs; Aether currently **Legacy/Multiboot path** — UEFI native: **UNKNOWN** |
| CSM | **UNKNOWN** |
| SATA mode option (AHCI/IDE/RAID) | **INFERRED** present in BIOS; exact menu labels **UNKNOWN** without photo of BIOS |
| Boot order | User uses USB/ISO; details **UNKNOWN** |
| ACPI DSDT/SSDT contents | **UNKNOWN** (not dumped) |
| PCI resource allocation by BIOS | **UNKNOWN** beyond standard |

---

## 4. Storage architecture (logical)

```
Physical SSD 0 ──┐
                 ├── SATA ports on HM76 ──► AHCI or IDE mode (BIOS)
Physical SSD 1 ──┘
                         │
            ┌────────────┴────────────┐
            │ AHCI (likely default)   │ IDE/Compat (if enabled)
            │ PCI 8086:1E03           │ PCI 8086:1E01/…
            │ ABAR MMIO               │ I/O 0x1F0/0x170
            └────────────┬────────────┘
                         ▼
                   Partition table (MBR/GPT)
                         ▼
                   Filesystem (NTFS typical for Windows; FAT possible)
                         ▼
                   Windows letters C:/D:  ← NOT physical disks
```

**User has two SSDs** → expect **two AHCI ports with devices**, each with own partition table. Letters C:/D: are OS mappings.

---

## 5. Other subsystems (brief)

| Subsystem | Expected | Confidence |
|-----------|----------|------------|
| xHCI USB 3.0 | Intel 7 Series xHCI on same PCH | DOC / INFERRED |
| EHCI USB 2.0 | Present | DOC |
| Wi-Fi | Intel 2230 PCI | DOC |
| Ethernet | RTL8111F PCI | DOC |
| HDA Audio | ALC269 on HDA bus | DOC |
| PS/2 kbd/touchpad | Works in Aether | **AETHER** |
| Framebuffer | Multiboot LFB 800×600 path works | **AETHER** |

---

## 6. Aether driver status vs AH532

| Driver | QEMU | AH532 |
|--------|------|-------|
| ATA PIO + FAT RO | PASS | FAIL (no device) |
| AHCI RO | NOT IMPLEMENTED | NOT VERIFIED |
| NVMe | N/A (no NVMe on this platform expected) | N/A |
| NTFS RO | NOT IMPLEMENTED | — |
| GPT | NOT IMPLEMENTED | — |

---

## 7. Required diagnostic on real AH532 (before writing AHCI driver)

Run Terminal command **`1`** (storage hardware diagnostics) and capture serial or on-screen:

1. Full PCI list (already partially available)
2. Every device with class `01` (mass storage): VID:DID, ProgIF, BARs
3. If ProgIF=AHCI and BAR5/ABAR present: dump GHC, CAP, PI, and per-port SSTS/SIG (**READ-ONLY**)
4. Legacy 0x1F0 status byte

**Do not implement full AHCI command engine until this dump is confirmed on AH532.**

---

## 8. Sources (documentation)

1. Fujitsu LIFEBOOK AH532 datasheets / factsheets (chipset HM76, SATA storage)
2. Notebookcheck review Lifebook AH532 (HM76, SATA HDD)
3. pci.ids — Intel 7 Series SATA `1e00`–`1e09`, AHCI `1e02`/`1e03`
4. Linux `ahci` PCI table — `0x1e03` Panther Point mobile AHCI
5. User Aether photos / reports (C: not detected, dual SSD)

---

## 9. Changelog

| Date | Change |
|------|--------|
| 2026-09-21 | Initial profile; ATA failure analysis; dual-SSD note; diagnostic plan |
