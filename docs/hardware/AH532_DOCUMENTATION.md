# Fujitsu LIFEBOOK AH532 — Hardware Documentation Reference

Persistent engineering reference for Aether OS. This is a research index, not a claim that every AH532 configuration is identical. Official documents, board-level references, Intel component documentation, and measurements from the actual Aether target are separated.

## 1. Fujitsu primary documentation

### Official product factsheets
- Fujitsu AH532/A532 factsheet: 15.6-inch 1366x768 display; HM76 family chipset; multiple CPU configurations; Intel HD Graphics 3000/4000 and optional NVIDIA GT620M configurations; SATA storage; Intel Centrino Wireless-N 2230; USB, HDMI, audio, Ethernet, ExpressCard and memory-card interfaces.
- Fujitsu regional/Russian product sheet also lists HM76 and Sandy Bridge/Ivy Bridge CPU options.
- Fujitsu explicitly warns that the factsheet describes general product specifications and does not guarantee a particular shipped configuration.

Official sources:
- https://www.fujitsu.com/downloads/COMP/fpcap/notebooks/previous/AH532_Factsheet.pdf
- https://www.fujitsu.com/ua/Images/RU_Product_Facts_Clients_10_12_l.pdf

### User/System Manual
- LIFEBOOK A532 / LIFEBOOK AH532 User Manual, Version 2, 23 Nov 2012.
- Fujitsu support lists English and multiple localized versions, including Russian.
- The manual covers physical hardware, ports, keyboard/touchpad, display, power, boot process, BIOS setup, storage, networking, regulatory information and specifications.

Official support:
- https://support.ts.fujitsu.com/Search/SWP1087701.asp
- Fujitsu AH532 support search: https://support.ts.fujitsu.com/IndexQuickSearchResult.asp?lng=RU&q=AH532

### BIOS Guide
- Fujitsu LIFEBOOK AH/LH Series BIOS Guide
- Models: AH532 / LH532
- Document date: 30 May 2012
- Part number: FPC58-3035-01
- Describes Info/System/Advanced/Security/Boot/Exit menus and firmware-controlled device features.

Reference copy/index:
- https://www.manualslib.com/manual/829337/Fujitsu-Lifebook-Ah532.html

## 2. Additional maintenance / repair documentation found

Searches found references to a Fujitsu service/repair/parts package for AH532 configurations. These are not treated as Fujitsu-public primary sources until independently verified.

One index explicitly lists:
- User manual
- Service manual
- Repair manual
- PartList

Reference:
- https://pdf-manuals.ru/fujitsu/ah532_i5_3210m_manuals.html

Another AH532 configuration index lists the same four document classes:
- https://pdf-manuals.ru/fujitsu/ah532_acsvj20026_manuals.html

### Motherboard-level documentation
Community repair references identify an AH532 board family:
- FH6 / FH6C
- Board marking: DA0FH6MB6E0
- Revision E is reported
- One schematic is named Fujitsu_FH6C_FH6_hm70_r0c_mb_0522.pdf
- The same package is reported as a Quanta ODM board.
- A separate community package is reported to contain 2 MB / 4 MB SPI flash images.

Sources:
- https://vinafix.com/threads/fujitsu-ah532-fh6-fh6c-hm70-r0c_mb_0522.15516/
- https://www.diy-laptoprepair.com/forum/fix-FUJITSU-AH532-FH6-FH6C-HM70-repair-guide-schematics.html

IMPORTANT: These board-level references may correspond to a particular AH532 board/configuration. Do not assume our physical machine has that exact board revision until confirmed by diagnostics or physical inspection.

## 3. Intel primary documentation relevant to this exact target

### Sandy Bridge / Intel HD Graphics 3000
Intel's public programmer reference collection for the 2011 Intel Core family contains the Sandy Bridge graphics documentation:
- Graphics Core
- Graphics Core MMIO, media registers and programming environment
- Render-engine memory interface and commands
- Video codec engine
- Blitter engine
- 3D/media pipeline
- VGA registers
- CPU display registers
- PCH display registers
- shared-function and message-gateway documentation

Intel reference:
- https://www.intel.com/content/www/us/en/docs/graphics-for-linux/developer-reference/1-0/intel-core-processor-2011.html

Relevant register-offset supplement:
- https://cdrdv2-public.intel.com/690991/snb-ihd-os-vol3-part3b-register-offsets.pdf

Relevant CPU display-register PRM:
- https://cdrdv2-public.intel.com/690986/snb-ihd-os-vol3-part2.pdf

Intel's current legacy-GPU documentation explicitly maps:
- device IDs including 0116
- Intel HD Graphics 3000
- Gen6
- Sandy Bridge

Reference:
- https://dgpu-docs.intel.com/overview/supported-hardware/legacy-gpus.html

Intel product specifications also identify device ID 0x116 as Intel HD Graphics 3000 and list eDP/DP/HDMI/SDVO/CRT output capability and two-display support for applicable SKUs.

## 4. Intel HM76 / Panther Point

Intel identifies HM76 as:
- Intel 7 Series chipset
- code name Panther Point
- mobile platform
- PCI Express 2.0
- up to 8 PCIe lanes
- 12 USB ports, including up to 4 USB 3.0
- 6 SATA ports, including up to 2 SATA 6 Gb/s
- Intel HD Audio
- Intel Rapid Storage Technology

Primary Intel source:
- https://www.intel.com/content/www/us/en/products/sku/64345/mobile-intel-hm76-express-chipset/specifications.html

For Aether, these are family/platform facts. Exact controller BARs, ACPI routing and PCI configuration on our physical machine must still come from runtime diagnostics.

## 5. Wireless

Verified physical target:
- Intel Centrino Wireless-N 2230
- PCI VID:DID 8086:0887
- subsystem 8086:4062

Intel documentation:
- PCIe half-mini card
- 2x2
- 2.4 GHz
- 802.11b/g/n
- up to 300 Mbps
- integrated Bluetooth
- PCIe system interface

Primary Intel source:
- https://www.intel.com/content/www/us/en/products/sku/66889/intel-centrino-wirelessn-2230-single-band/specifications.html

Intel regulatory/documentation index:
- https://www.intel.com/content/www/us/en/support/articles/000007443/wireless/legacy-intel-wireless-products.html

## 6. Audio / LAN

Fujitsu documentation identifies:
- Realtek ALC269 HD Audio codec
- built-in stereo speakers
- digital microphone
- 3.5 mm headphone and microphone interfaces
- 10/100/1000 Ethernet
- Realtek LAN is identified in some detailed sheets as RTL8111F.

The exact physical target PCI ID already observed by Aether diagnostics is:
- Ethernet vendor/device 10ec:8168

Do not equate the family datasheet's RTL8111F wording with the exact silicon revision without PCI/subsystem identification.

## 7. USB

Physical diagnostics:
- Intel xHCI controller
- PCI VID:DID 8086:1E31
- physical PS/2 touchpad works
- physical USB mouse was not yet working in recorded Aether diagnostics
- USB keyboard works in QEMU

The Fujitsu/Intel platform documentation describes USB 3.0/2.0 support; exact port-to-controller routing must be established from PCI/ACPI diagnostics.

## 8. Display / GPU facts specific to the actual Aether machine

Physical verification:
- Intel HD Graphics 3000
- Sandy Bridge
- PCI VID:DID = 8086:0116
- Gen6
- KMS/modeset works
- stable Aether desktop at 800x600
- VBlank works
- PIPE0 STAT=0, DSL=62
- MMIO base = 0xF0000000
- stolen memory base = 0xDBA00000
- reported GTT physical region = 0xDF800000
- GTT size = 2 MB
- aperture = 256 MB

### Critical safety finding
Direct mapping/access around physical 0xDF800000 caused a real AH532 reboot during gmm/gm2 experiments.

Therefore:
- 0xDF800000 is a DO-NOT-TOUCH region until new evidence exists.
- Do not re-enable GGTT/GSM mapping merely because a QEMU experiment succeeds.
- Prefer read-only MMIO/KMS diagnostics and controlled EDID/GMBUS work.
- Never interpret QEMU success as proof of physical AH532 safety.

## 9. Storage

Physical Aether diagnostics:
- SSD 240 GB class, reported 228936 MB
- ST9160411AS, reported 152627 MB
- AetherFS RAM system volume A:
- observed partitions include NTFS/ExFAT, Linux, EFI and UNKNOWN

Aether policy:
- host-disk writes prohibited
- NTFS currently read-only/limited
- ext4 not implemented
- MBR/GPT detection exists

## 10. BIOS / firmware

Known references:
- BIOS Guide dated 30 May 2012, part FPC58-3035-01
- community reports exist for later AH532 BIOS versions including 2.14 and 2.18, but these are not treated as the exact BIOS of our machine without runtime evidence.
- Aether should obtain the actual BIOS vendor/version/date and ACPI information from the target before relying on firmware assumptions.

## 11. Known-good Aether baseline

Physical AH532 has demonstrated:
- GRUB/Multiboot2 BIOS/CSM boot
- x86_64 long mode
- PMM/paging/GDT/IDT
- existing framebuffer
- AetherFS RAM disk
- ATA/AHCI read-only storage
- Ring3/userspace path
- separate USER_CR3 from KERNEL_CR3
- process management
- PS/2 input
- XP-style desktop
- stable 800x600 graphics

## 12. QEMU rule

Every experiment must be tagged:
- QEMU-only
- physical AH532 verified
- documentation-derived
- inferred/unknown

QEMU is an automated laboratory, not a perfect AH532 emulator.

In particular, a QEMU graphics result cannot validate an Intel Gen6 register sequence or physical-memory mapping on AH532.

## 13. Next hardware evidence to collect

Before deeper driver work, collect from the real machine:
1. complete PCI configuration/BAR dump
2. CPU CPUID and exact model
3. PCH/chipset PCI IDs
4. ACPI tables and device topology
5. actual BIOS version/date
6. GPU PCI command/status/BARs
7. display connector/EDID information
8. xHCI BAR and capability registers
9. AHCI controller PCI/BAR information
10. SMBIOS/DMI identifiers useful for hardware matching without exposing unnecessary personal data
11. exact board marking/revision if physically visible

This list is deliberately separate from assumptions. The goal is to make the QEMU laboratory and Aether drivers reflect the actual machine rather than a generic AH532.

## 14. Research status

This is a substantially expanded research index, but it is NOT honest to call it “all documentation”. Some Fujitsu service/repair material appears to be distributed through third-party archives, some board-level files are configuration-specific, and some firmware/board information must be obtained from the physical unit.

The highest-value primary references currently found are Fujitsu's AH532 documentation and Intel's Sandy Bridge Gen6 graphics PRMs. The board-level schematic/repair references are retained as leads, not as verified facts about our exact machine.
