# Fujitsu LIFEBOOK AH532 — Hardware Documentation Reference

Persistent engineering reference for Aether OS. Official documentation, physical diagnostics, and unknown/inferred facts are kept separate.

## Official Fujitsu documentation
- Fujitsu AH532/A532 factsheet: 15.6-inch 1366x768 display; Intel HM76 family chipset; multiple CPU configurations; Intel HD Graphics 3000 and HD Graphics 4000 are both listed as possible family configurations; SATA storage; Intel Centrino Wireless-N 2230; USB, HDMI, audio, Ethernet, ExpressCard and memory-card interfaces.
- Fujitsu User/System Manual: LIFEBOOK A532 / LIFEBOOK AH532, Version 2, dated 23 Nov 2012. English PDF is listed by Fujitsu; localized versions including Russian are also listed.
- Fujitsu support portal provides AH532/A532 documentation, drivers and BIOS; some downloads require product/serial identification.

## Actual Aether target — physical Fujitsu LIFEBOOK AH532
These values come from diagnostics on the real machine and take precedence over generic family specifications when they conflict.

### GPU
- Intel HD Graphics 3000
- Sandy Bridge
- PCI VID:DID = 8086:0116
- Intel Gen6
- KMS/modeset works
- 800x600 desktop is stable
- VBlank works; PIPE0 STAT=0, DSL=62
- MMIO base = 0xF0000000
- stolen memory base = 0xDBA00000
- GGTT/GTT physical region reported at 0xDF800000
- GTT size = 2 MB
- aperture = 256 MB

### Critical GPU safety finding
- Direct mapping/access around physical 0xDF800000 caused a real AH532 reboot during gmm/gm2 experiments.
- 0xDF800000 is currently a DO-NOT-TOUCH region.
- Do not re-enable GGTT/GSM mapping merely because it works in QEMU.
- Future GPU work must use safe MMIO/KMS/read-only diagnostics first.
- QEMU success is not proof of AH532 safety.

### Wireless
- Intel Centrino Wireless-N 2230
- PCI VID:DID = 8086:0887
- subsystem = 8086:4062

### Ethernet
- Realtek Ethernet
- PCI vendor/device = 10ec:8168

### USB
- Intel xHCI controller, PCI VID:DID = 8086:1E31
- Physical PS/2 touchpad works.
- Physical USB mouse was not yet working in recorded diagnostics.

### Storage
- SSD: 240 GB class, reported 228936 MB
- ST9160411AS: reported 152627 MB
- AetherFS RAM system volume A:
- Observed partitions include NTFS/ExFAT, Linux, EFI and UNKNOWN partitions.
- Host disks must remain read-only; no host-disk writes.
- NTFS is currently read-only and limited; ext4 is not implemented.

## Aether known-good baseline on AH532
- GRUB/Multiboot2 BIOS/CSM boot
- x86_64 long mode, PMM, paging, GDT/IDT
- existing framebuffer
- AetherFS RAM disk
- ATA/AHCI read-only storage
- Ring3/userspace path and process management
- PS/2 input
- XP-style desktop
- stable 800x600 graphics

## QEMU interpretation rule
Every result must be labelled QEMU-only, physical AH532 verified, or inferred from documentation. A passing QEMU run must never be used as evidence that a GPU register sequence or memory mapping is safe on AH532.

## Development priorities
1. Preserve working 800x600/KMS.
2. Use read-only video diagnostics.
3. Continue Gen6/Sandy Bridge-specific work.
4. Investigate EDID/GMBUS and safe mode information.
5. Avoid direct 0xDF800000 access until there is new evidence and a controlled test plan.
6. Collect PCI/BAR/ACPI information from the physical machine before implementing new drivers.

## Source classification
Official = Fujitsu product factsheets and Fujitsu support documentation.
Physical verification = diagnostics from the real Fujitsu AH532.
Unknown/inferred = anything not directly observed on the physical machine or explicitly documented by Fujitsu.

This document is an engineering reference and does not replace runtime diagnostics from the actual machine.