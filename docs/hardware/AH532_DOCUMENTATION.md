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


## 15. Deeper board / firmware findings (research pass)

### Board identity: stronger but still not physical-unit proof
Multiple independent repair indexes now converge on an AH532/A532 board family:
- Quanta ODM
- FH6 / FH6C
- board marking DA0FH6MB6E0
- Rev E is repeatedly listed
- schematic filename: Fujitsu_FH6C_FH6_hm70_r0c_mb_0522.pdf

The same community technical package identifies Realtek ALC269 audio and Realtek RTL8111F LAN on this board family. It also publishes SHA-256 hashes for the referenced schematic/firmware package.

Sources:
- RepairLap: https://www.repairlap.com/threads/fujitsu-lifebook-ah532-schematic-da0fh6mb6e0-rev-e-bios.5293/post-8615
- Vinafix: https://vinafix.com/threads/fujitsu-ah532-fh6-fh6c-hm70-r0c_mb_0522.15516/
- Alex Laptop Repair: https://www.alexlaptoprepair.com/forums/threads/fujitsu-lifebook-a532-ah532-da0fh6mb6e0-rev-e-schematic.5056/

IMPORTANT CONFLICT: the community schematic/package is labelled HM70 in its filename, while Fujitsu product documentation and the AH532 configuration relevant to this project identify HM76-family systems. Therefore the schematic must NOT be treated as the exact electrical map of our machine until the physical board marking and runtime PCH PCI ID are confirmed.

### SPI / firmware architecture lead
A separate repair report for an A532/AH532-family board describes three SPI devices:
- U24: 1 MB, described as EC firmware
- U35: 4 MB, main BIOS
- U7: 2 MB, Intel Management Engine region/device

Other repair indexes independently list the same U35/U7/U24 sizes for DA0FH6MB6E0 Rev E. This is useful evidence for understanding the firmware topology, but it is third-party repair evidence and must not be used to write/flash anything on the Aether machine.

Sources:
- https://winraid.level1techs.com/t/realtek-network-problem-on-fujitsu-a532/36245
- https://www.bios-downloads.com/product-category/bios/laptop-bios/fujitsu-laptop-bios/page/18/

### HM76 PCI identity
Intel documentation identifies mobile HM76 LPC as PCI device ID 8086:1E59. The broader Intel 7-Series/Panther Point documentation describes HM76 as a 12-USB/6-SATA mobile PCH with up to 8 PCIe 2.0 lanes.

Reference:
- https://www.intel.com/content/www/us/en/docs/dynamic-application-loader/developer-guide/1-0/for-api-level-2-intel-me-8-0-ivy-bridge.html
- https://www.intel.com/content/dam/www/public/us/en/documents/datasheets/7-series-chipset-pch-datasheet.pdf

For Aether this gives us a concrete runtime check: the real machine should expose the HM76 LPC function as 8086:1E59 if the HM76 assumption is correct. We should verify this in the next PCI diagnostic rather than hard-code it from documentation.

### BIOS guide details worth preserving
The AH/LH BIOS Guide explicitly documents AH532/LH532 firmware menus and states that the System/Advanced menus expose storage and device-feature controls, including the Serial ATA controller and USB-related options. It also notes that displayed fields vary with system configuration.

Reference:
- https://www.manualslib.com/manual/829337/Fujitsu-Lifebook-Ah532.html

This matters to Aether because ACPI/firmware routing should be discovered from the actual machine rather than inferred only from the product factsheet.

## 16. Hardware-map research matrix

| Subsystem | Documentation | Runtime target evidence | Aether status | Main unknown |
|---|---|---|---|---|
| CPU | Intel Sandy Bridge/Ivy Bridge platform docs | CPUID still needed | booted x86_64 | exact model/features |
| PCH/HM76 | Intel 7-Series PCH datasheet | expected LPC ID 8086:1E59; verify | PCI discovery exists | full function/BAR/ACPI map |
| HD Graphics 3000 | Intel SNB Gen6 PRMs | 8086:0116, Gen6, KMS 800x600 verified | partial display driver | safe scanout/EDID/advanced acceleration |
| Display panel/connector | Fujitsu manual + Intel display PRM | EDID/connector dump still needed | framebuffer/KMS | exact panel/link topology |
| PCIe root ports | Intel PCH docs | full root-port dump needed | partial PCI | exact routing to Wi-Fi/LAN/etc. |
| xHCI | Intel PCH docs | 8086:1E31 observed | controller found | BAR/capabilities/port routing |
| AHCI/SATA | Intel PCH docs + Fujitsu manual | physical disks observed | read-only storage | exact AHCI PCI/BAR/port map |
| LAN | Fujitsu/board references + Realtek family docs | 10EC:8168 observed | not a complete native driver | exact revision/subsystem/PHY |
| Wi-Fi/BT | Intel 2230 docs | 8086:0887, subsystem 8086:4062 | not implemented | PCI BAR/firmware/transport/runtime state |
| HDA/ALC269 | Fujitsu/board references | codec/controller runtime dump needed | not implemented | exact HDA codec/link routing |
| SMBus | Intel PCH docs | runtime PCI function needed | not mapped | SMBus BAR/addressing and devices |
| EC | board/firmware repair evidence | runtime/ACPI evidence needed | not implemented | exact EC model/register interface |
| ACPI | Fujitsu BIOS + actual firmware | table dump needed | partial assumptions only | DSDT/SSDT/GPE/device topology |
| SPI/BIOS/ME | third-party board repair evidence | BIOS version/date needed | read-only awareness | exact flash layout on physical board |
| Card reader | Fujitsu product docs; board family likely Realtek | runtime PCI ID needed | unknown | controller/device ID |
| Camera | Fujitsu product docs | USB topology needed | unknown | exact USB device/interface |
| Audio jacks/speakers/mic | Fujitsu manual | HDA + GPIO/codec evidence needed | unknown | exact jack/GPIO routing |
| Power/battery/thermal | Fujitsu manual + ACPI | ACPI/EC/battery dump needed | unknown | EC/thermal zones/fan control |

## 17. New rule for exact-hardware matching

Do not identify the physical AH532 from the model name alone. For any board-level assumption, require at least one of:
1. matching runtime PCI IDs/subsystem IDs;
2. matching DMI/SMBIOS board identifiers;
3. physical board marking/revision;
4. a firmware/ACPI identifier that uniquely matches the board family.

Until then, board schematics are research leads only.


## 18. Controller-level documentation pass

### Intel 7-Series PCH function map
Intel's 7-Series PCH specification update gives a useful exact device/function map for the family:
- D31:F2 SATA: mobile AHCI device ID 1E03 (with configuration-dependent alternatives)
- D31:F3 SMBus: device ID 1E22
- D31:F6 Thermal: device ID 1E24
- D29:F0 EHCI #1: 1E26
- D26:F0 EHCI #2: 1E2D
- D20:F0 xHCI: 1E31
- D27:F0 Intel HD Audio: 1E20
- D28:F0/F1/F2 PCIe root ports: 1E10 / 1E12 / 1E14 in the normal mobile configuration

Primary Intel sources:
- https://www.intel.com/content/dam/www/public/us/en/documents/specification-updates/7-series-chipset-pch-spec-update.pdf
- https://www.intel.com/content/dam/www/public/us/en/documents/datasheets/7-series-chipset-pch-datasheet.pdf

This is especially useful for the next Aether PCI diagnostic: one scan can determine whether the physical machine's PCH topology matches the expected family instead of assuming it.

### SMBus: concrete register target
Intel documents the SMBus controller at D31:F3. Its PCI configuration includes:
- SMBMBAR1 at offset 14h-17h for a 64-bit memory base field
- SMB_BASE at offset 20h-23h
- SMB_BASE is I/O mapped, with the base encoded in bits 15:5

Source: Intel 7-Series PCH datasheet, SMBus controller registers.

For Aether this means the first SMBus step should be strictly read-only PCI configuration discovery: VID/DID, command/status, BAR/base, revision and interrupt routing. Only after that should any SMBus transaction engine be considered.

### xHCI: concrete PCI discovery targets
Intel documents the xHCI controller at D20:F0 and gives its PCI header fields, including command/status, revision, subsystem IDs and the MMIO BAR at offset 10h-17h. The physical AH532 already reports 8086:1E31, so this is a strong match between documentation and target hardware.

Source: Intel 7-Series PCH datasheet.

The next xHCI diagnostic should therefore collect the complete PCI header and xHCI capability registers without enabling ports or starting DMA. This is safer than jumping directly into USB HID work.

### AHCI/SATA: exact family target
Intel's specification update identifies mobile AHCI as D31:F2 / device ID 1E03 for the relevant 7-Series mobile configurations. The physical Aether system already exposes working ATA/AHCI storage, so the next useful evidence is the controller's PCI command/status, BARs and AHCI CAP/PI registers.

No write operations are needed: the controller can be characterized with read-only PCI and AHCI capability reads.

### HD Audio / ALC269
Intel identifies the PCH HD Audio controller as D27:F0 / 1E20. Linux's HD-Audio documentation confirms the architecture: a controller plus one or more codecs on the HD-audio bus, with codec-specific handling for Realtek ALC269-class devices. The Linux documentation also records ALC269-specific laptop/digital-mic/headset fixup categories.

Sources:
- https://www.kernel.org/doc/html/latest/sound/hd-audio/notes.html
- https://cdn.kernel.org/doc/html/latest/sound/hd-audio/models.html

For Aether, the first audio milestone should be controller discovery and codec enumeration only. We need the actual codec vendor/device ID and NID topology from the physical AH532 before implementing playback or GPIO/jack handling.

### Realtek LAN
The physical controller is known only at the PCI-family level so far: 10EC:8168. The exact RTL8168/RTL8111 revision and subsystem still need to be read from PCI configuration. Do not select a register sequence based only on the marketing name RTL8111F.

The safest next step is a complete read-only PCI header/BAR/revision dump, followed by PHY identification. TX/RX DMA should come only after the MMIO/DMA addressability assumptions are independently verified.

## 19. ACPI / EC / thermal research boundary

No trustworthy public source found in this pass provides a complete, machine-specific AH532 DSDT/SSDT set that can be treated as the firmware of our physical unit. Therefore Aether should not hard-code EC addresses, fan registers, battery registers or GPIO mappings from a random AH532 dump.

The correct next evidence is obtained from the physical firmware:
1. RSDP/XSDT/RSDT addresses and checksums;
2. enumerate DSDT/SSDT tables with OEM ID, OEM Table ID and revision;
3. locate EC device(s), thermal zones, battery/AC adapters and fan-related methods;
4. record PCI _ADR/_STA/_CRS relationships for xHCI, SATA, HDA and PCIe devices;
5. record GPIO/SMBus dependencies only when explicitly described by ACPI;
6. keep all table parsing read-only.

This is a deliberate boundary: documentation tells us what the chipset can contain, while ACPI on the actual machine tells Aether how Fujitsu wired/configured it.

## 20. Revised next hardware diagnostic set

The highest-value single diagnostic pass on the physical AH532 is now:

### PCI/PCH
- scan bus 0 functions, especially D20-D31;
- record VID:DID, revision, class, command/status, subsystem IDs;
- record every BAR without mapping or touching the MMIO contents.

### PCH-specific expected IDs to compare
- D31:F2 SATA: 8086:1E03 expected for mobile AHCI family;
- D31:F3 SMBus: 8086:1E22;
- D31:F6 Thermal: 8086:1E24;
- D20:F0 xHCI: 8086:1E31 (already observed);
- D27:F0 HDA: 8086:1E20;
- D28:F0/F1/F2 PCIe: 8086:1E10/1E12/1E14 in the normal mobile configuration.

### Then ACPI
- RSDP/XSDT/RSDT;
- DSDT/SSDT identity metadata;
- device names and _ADR/_CRS for the controllers above;
- EC, thermal, battery and fan objects.

### Then individual controller probes
1. xHCI capabilities, read-only;
2. AHCI CAP/PI, read-only;
3. HDA controller + codec enumeration, read-only;
4. RTL8168 PCI revision/subsystem/PHY identification, read-only;
5. SMBus controller discovery, no transactions initially.

This ordering minimizes risk and gives Aether a real hardware map before writing additional drivers.


## 21. Fujitsu support-package inventory (additional evidence)

The official Fujitsu AH532 support search exposes more than the user manual and BIOS guide. The historical driver catalogue identifies the software components Fujitsu shipped for this platform, which is useful as a hardware inventory cross-check:

- Intel Management Engine Interface
- Intel Rapid Storage Technology
- Intel Chipset Device Software
- Intel Wireless LAN
- Qualcomm Atheros Wireless LAN / Bluetooth for alternate AH532 configurations
- Realtek PCIe GBE/FE Family Controller LAN
- Realtek High Definition Audio
- Intel USB 3.0 Host Controller
- Realtek RTS5170 Memory Card reader
- Sonix Camera
- Synaptics Pointing Device / ALPS Flat Point on different configurations
- Intel Display Driver
- NVIDIA Display Driver on discrete-GPU configurations
- Fujitsu FUJ02B1/FUJ02E3 platform-device drivers
- Wireless Radio Switch Driver
- WIDCOMM Bluetooth Software on some configurations

Official Fujitsu search results:
- https://support.ts.fujitsu.com/IndexQuickSearchResult.asp?OpenTab=&Q=LIFEBOOK+AH532&lng=
- https://support.ts.fujitsu.com/IndexQuickSearchResult.asp?lng=RU&q=AH532

IMPORTANT: these packages describe supported product configurations, not necessarily the exact hardware fitted to our machine. They are evidence for candidate controllers, not proof of presence.

### Particularly useful new controller lead: Realtek RTS5170
Fujitsu's AH532 support catalogue explicitly lists the Realtek RTS5170 Memory Card Driver. This gives us a concrete candidate for the SD/memory-card controller that was previously only marked unknown. The physical PCI/USB inventory must still identify the actual controller before a driver is written.

### Camera lead: Sonix
The same official catalogue lists a Sonix Camera driver. This is a useful candidate identification for the integrated webcam, but the exact USB VID/PID must be obtained from the physical machine.

### Platform-management leads: FUJ02B1 / FUJ02E3
Fujitsu ships dedicated FUJ02B1 and FUJ02E3 device drivers for AH532-family systems. These are important clues that some notebook-specific platform/ACPI functionality is exposed through Fujitsu-specific ACPI devices. Aether should therefore enumerate ACPI hardware IDs before attempting to reproduce any of those functions.

### Alternate wireless configurations
Fujitsu's catalogue contains both Intel Wireless LAN and Qualcomm Atheros Wireless LAN/Bluetooth packages. Our physical machine has already identified Intel Centrino Wireless-N 2230 8086:0887 with subsystem 8086:4062, so the Atheros packages should be treated as alternate configuration evidence, not as evidence against the Intel card.

## 22. BoardView / schematic availability

The research pass confirms that a board-level schematic exists for the AH532/A532 Quanta FH6/FH6C family and that boardview files have also circulated for the same board marking. Multiple sources identify:
- DA0FH6MB6E0 Rev E
- Quanta FH6/FH6C
- 45-page schematic listings in commercial archives
- a free/community maintenance-guide discussion that explicitly mentions PDF + FZ boardview files

Sources:
- https://www.realschematic.com/shop/9971/desc/fujitsu-lifebook-ah532-a532
- https://www.diy-laptoprepair.com/forum/fix-FUJITSU-AH532-FH6-FH6C-HM70-repair-guide-schematics.html
- https://www.alexlaptoprepair.com/forums/threads/fujitsu-lifebook-a532-ah532-da0fh6mb6e0-rev-e-schematic.5056/

The boardview is potentially the most valuable missing board-level artifact because it can map controller pins, power rails, connectors, EC/GPIO relationships and component designators. However, it must only be applied after confirming the physical board marking/revision.

## 23. Documentation completeness status — explicit

We are NOT yet entitled to call the documentation complete.

What is now covered well:
- Fujitsu primary product/manual/BIOS documentation;
- historical Fujitsu driver/package inventory;
- Intel Sandy Bridge/Gen6 GPU programming references;
- Intel 7-Series/Panther Point chipset references;
- controller-level PCI IDs/register-map leads;
- board/schematic/BIOS/EC community references;
- boardview availability;
- physical AH532 runtime facts already measured by Aether;
- explicit separation of verified, documented, alternate-configuration and unknown data.

What remains potentially missing and worth another dedicated pass:
1. the actual FH6/FH6C schematic PDF contents, page by page;
2. the matching boardview/FZ component database, if legally/technically obtainable;
3. exact Fujitsu BIOS/EC package versions for the physical machine;
4. ACPI/DSDT/SSDT dumps from the physical machine;
5. exact PCI subsystem/revision data for every controller;
6. Intel ME/firmware documentation relevant to the detected PCH generation;
7. detailed datasheets/programming manuals for the exact Realtek LAN, HDA codec and card reader revisions;
8. exact panel/eDP/LVDS/EDID topology;
9. EC/GPIO/keyboard/touchpad power-management relationships;
10. service-part numbers and connector pinouts.


## 24. Second-pass source audit: additional findings

A dedicated second-pass search found additional evidence that should remain in the engineering record.

### Fujitsu official support catalogue
The official Fujitsu AH532 support search is substantially richer than a simple driver list. It records historical versions of Intel Management Engine Interface, Intel Rapid Storage Technology, Realtek High Definition Audio, Fujitsu BIOS Driver, FUJ02B1 and FUJ02E3 platform-device drivers, Intel Wireless LAN and Intel Bluetooth, Wireless Radio Switch, Sonix Camera, ALPS Flat Point and Synaptics pointing-device packages, Intel Chipset Device Software, and NVIDIA display drivers for discrete-GPU configurations.

This confirms that the AH532 product family had multiple hardware/configuration variants. These entries are useful as candidate-hardware evidence, but must not override physical PCI/USB/ACPI evidence from our machine.

Official source:
- https://support.ts.fujitsu.com/IndexQuickSearchResult.asp?OpenTab=&Q=LIFEBOOK+AH532&lng=

### Linux hardware-probe cross-check
A public Linux hardware probe for an AH532/G-series machine provides a useful independent configuration cross-check. It reports a Samsung 1366x768 LCD panel, BIOS 2.14 dated 2018-09-07, Intel Core i3-3110M, Patriot DDR3-1600 4GB memory, USB 3.0 root hub, Intel integrated-rate-matching USB hub, and a USB Sigma Micro XM102K mouse.

This is not proof of our exact AH532 configuration, but it demonstrates that the model exposes the expected platform classes to a real OS and gives us a reference for future Aether diagnostics.

Source:
- https://linux-hardware.org/?probe=d00301ccac

### Independent Linux driver-stack evidence
A real AH532/G21 Linux report shows the expected driver ecosystem around this platform, including i915, iwlwifi, snd_hda_intel, snd_hda_codec_realtek, r8169, xhci, uvcvideo, mei, fujitsu_laptop, i2c_i801 and serio_raw. This is useful as a discovery checklist only; it does not establish exact Aether register mappings.

Source:
- https://forum.rosa.ru/viewtopic.php?sid=5950eef09ee5e167506824654fa67b5c&t=10017

### Board-level evidence strengthened
Multiple independent repair archives identify the same board family: Quanta FH6/FH6C, DA0FH6MB6E0 Rev E, a 45-page schematic, and U35/U7/U24 firmware regions associated with the board. One repair archive also identifies the schematic block-diagram controller set as Realtek ALC269 audio and RTL8111F LAN. This is consistent with Fujitsu product documentation, but the exact silicon revision and subsystem IDs still need to come from our physical PCI scan.

Sources:
- https://www.repairlap.com/threads/fujitsu-lifebook-ah532-schematic-da0fh6mb6e0-rev-e-bios.5293/post-8614
- https://vinafix.com/threads/fujitsu-ah532-fh6-fh6c-hm70-r0c_mb_0522.15516/
- https://realschematic.com/shop/9971/desc/fujitsu-lifebook-ah532-a532

### Important board/chipset ambiguity retained
The community schematic filename contains hm70, while Fujitsu/Intel product documentation and our target-platform assumptions point toward HM76 / 7-Series mobile PCH. This discrepancy is explicitly retained as unresolved. It may reflect schematic naming, a related board variant, or a genuine configuration distinction. Aether must identify the actual PCH by PCI ID before using board-level assumptions.

### BIOS evidence
Community firmware archives identify AH532/FH6 firmware components as U35 main SPI, U7 ME-related SPI and U24 EC SPI. Another independent dump record identifies the same DA0FH6MB6E0 Rev E board and the three flash devices. These are documentation/research references only. No firmware write operation is part of Aether development.

Sources:
- https://www.repairlap.com/threads/fujitsu-lifebook-ah532-schematic-da0fh6mb6e0-rev-e-bios.5293/post-8615
- https://remont-aud.net/dump/kompjutery_noutbuki_netbuki/fujitsu/fujitsu_lifebook_ah532_g52_shassi_main_board_da0fh6mb6e0_rev_e/433-1-0-57624

## 25. Source-exhaustion rule

This document should not be treated as literally containing every Internet page about AH532. The practical definition of completion is now:

1. primary Fujitsu documentation and support inventory covered;
2. Intel CPU/GPU/PCH primary documentation covered;
3. board-level schematic/board-family evidence cross-checked across independent archives;
4. BIOS/EC/ME evidence indexed without treating third-party firmware as authoritative;
5. independent Linux hardware probes used only as cross-checks;
6. every important unknown assigned a concrete physical diagnostic needed to resolve it;
7. conflicting claims explicitly preserved instead of silently merged;
8. no unsafe hardware experiment inferred from documentation alone.

The remaining high-value work is therefore no longer broad web searching alone. It is obtaining the missing primary artifacts where accessible (especially the actual schematic/boardview contents) and collecting machine-specific runtime evidence: PCI configuration space, ACPI tables, controller BARs, exact revisions/subsystem IDs, EDID and panel identity, and EC/firmware identity.


## 26. Third-pass audit: board identity and firmware evidence

The third-pass search found stronger evidence for the physical-board research target, while also clarifying what remains unverified.

### DA0FH6MB6E0 Rev.E is repeatedly associated with AH532/G52
RepairLap identifies Fujitsu LIFEBOOK AH532 with Quanta FH6 and motherboard DA0FH6MB6E0 Rev.E, and lists separate U35, U7 and U24 firmware images. The same source also lists a schematic named Fujitsu_FH6C_FH6_hm70_r0c_mb_0522.pdf.

Sources:
- https://www.repairlap.com/threads/fujitsu-lifebook-ah532-schematic-da0fh6mb6e0-rev-e-bios.5293/post-8615
- https://www.repairlap.com/threads/fujitsu-lifebook-ah532-schematic-da0fh6mb6e0-rev-e-bios.5293/post-8614

A separate repair database reports a working AH532/G52 firmware read from a DA0FH6MB6E0 Rev.E board, naming three SPI devices: U35, U7 and U24. It reports an Intel i3-3110M for that particular dumped machine. This is strong board-level corroboration but still not proof that our physical AH532 has the same CPU or firmware revision.

Source:
- https://remont-aud.net/dump/kompjutery_noutbuki_netbuki/fujitsu/fujitsu_lifebook_ah532_g52_shassi_main_board_da0fh6mb6e0_rev_e/433-1-0-57624

### Board schematic contents are still the key missing primary artifact
Search results consistently expose the schematic as a downloadable 1.4–1.5 MB PDF, but the accessible web index does not expose its page contents. Therefore we should not invent register/pin information from its filename or block-diagram summary. The exact PDF contents should be obtained and inspected if legally and technically accessible.

### BoardView availability strengthened
A public schematic/boardview archive explicitly lists `DA0FH6MB6E0 rev E PDF` under both `#SCHEMATIC` and `#BOARDVIEW`. This establishes that boardview data has circulated for the exact board marking, but the archive page does not expose the actual boardview contents in searchable text.

Source:
- https://t.me/s/schematicslaptop?before=10583

### Fujitsu support inventory: platform-specific devices
The official Fujitsu catalogue gives concrete version/date evidence for the platform-device drivers. For example, FUJ02B1 driver 1.23 and FUJ02E3 driver 1.30.3 are listed with 2016 publication dates, and earlier 2013 versions are also present. This strengthens the case for enumerating Fujitsu-specific ACPI device IDs on the physical machine rather than assuming generic ACPI behavior.

Source:
- https://support.ts.fujitsu.com/IndexQuickSearchResult.asp?lng=COM&q=fujitsu+ah532

### Realtek card-reader evidence
The official Fujitsu catalogue explicitly lists Realtek RTS5170 Memory Card Driver 6.2.9200.39048. This gives us a concrete candidate for the memory-card controller. It remains a candidate until the physical machine's PCI/USB enumeration identifies the actual device.

Source:
- https://support.ts.fujitsu.com/IndexQuickSearchResult.asp?OpenTab=&Q=LIFEBOOK+AH532&lng=

### New physical diagnostic priority
The strongest remaining information gap is no longer generic documentation. It is exact machine identity. The next Aether hardware pass should collect, read-only:
1. complete PCI configuration for bus 0 functions D20-D31;
2. PCH ID/revision and all relevant subsystem IDs;
3. BAR addresses without mapping their MMIO contents;
4. xHCI capability registers only after identifying its BAR safely;
5. AHCI CAP/PI;
6. HDA controller identity and codec enumeration;
7. Realtek LAN revision/subsystem and PHY identity;
8. SMBus controller identity and base address, with no SMBus transactions initially;
9. ACPI RSDP/XSDT/RSDT plus DSDT/SSDT table identities;
10. EDID/panel identity through the already-working display path;
11. USB topology including camera/card-reader candidates.

This diagnostic list is deliberately read-only and does not authorize MMIO mapping, DMA activation, firmware writes, or access to the known-unsafe GTT physical address 0xDF800000.


## 27. Fourth-pass audit: exact board-package fingerprints

The fourth-pass search produced useful fingerprints for the board package, even though the actual schematic/boardview payload remains behind archive download access.

### Exact schematic fingerprint
Vinafix identifies the document as a **Discrete Block Diagram**, manufacturer/ODM Quanta, model FH6C, motherboard DA0FH6MB6E0, with key controllers Realtek ALC269 audio and Realtek RTL8111F LAN. It reports the schematic PDF SHA-256 as:
`8011b1775a403b5d62ff816c278412f62c2e3d7873b955f2721fa664528badbc`
The listed file size is 1.4 MB. This gives us a concrete fingerprint to use if the PDF is obtained elsewhere.

Source:
- https://vinafix.com/threads/fujitsu-ah532-fh6-fh6c-hm70-r0c_mb_0522.15516/

### Exact firmware-package fingerprint
The same archive identifies `DAOFH6MB6E0 REV E.zip` as a 3.6 MB SPI-NOR firmware package with SHA-256:
`871ad6af3c87269a84276846dba44c9d620e3e2161b8483372f1fc7d497c6409`
Its published contents include `2m.bin` and `4m.bin`. RepairLap separately lists U35, U7 and U24 images for the same board. These are valuable research fingerprints, but Aether must never flash them merely because they match the board marking.

Sources:
- https://vinafix.com/threads/fujitsu-ah532-fh6-fh6c-hm70-r0c_mb_0522.15516/
- https://www.repairlap.com/threads/fujitsu-lifebook-ah532-schematic-da0fh6mb6e0-rev-e-bios.5293/post-8618

### Strong evidence that U35/U7/U24 are separately archived
A repair thread contains distinct old-file entries named `fh6u35old.BIN`, `fh6u7old.BIN`, and `fh6u24old.BIN`, and later an AH532 DA0FH6MB6E0 Rev.E set containing 2 MB, 4 MB and 1 MB images. This reinforces the three-device firmware layout seen in other sources.

Source:
- https://vinafix.com/threads/fujitsu-ah532-fh6-fh6c-hm70-r0c_mb_0522.15516/page-2

### BoardView status remains unresolved
The Telegram archive explicitly lists `DA0FH6MB6E0 rev E PDF .rar` as both `#SCHEMATIC` and `#BOARDVIEW`, with a 679.4 KB archive. However, the public indexed page does not expose the contained boardview data. We therefore record availability, but do not claim to have inspected the boardview itself.

Source:
- https://t.me/s/schematicslaptop?before=10583

### Current conclusion from the source audit
We now have independent confirmation of:
- model/family relationship: Fujitsu AH532 ↔ Quanta FH6/FH6C;
- exact board marking: DA0FH6MB6E0 Rev.E;
- schematic filename and cryptographic fingerprint;
- boardview archive existence;
- firmware archive fingerprint and component layout;
- candidate LAN/audio silicon;
- official Fujitsu driver inventory;
- independent Linux hardware/driver cross-checks.

The remaining documentary gap is therefore specifically the **actual contents** of the board schematic and BoardView, plus machine-specific firmware/ACPI identity. Searching further for duplicate index pages is unlikely to add much unless it exposes the actual files or new primary evidence.


## 28. Fourth-pass result: exact-file availability and board-family cross-reference

A further search confirms that the exact schematic is indexed by multiple independent repair repositories, but the actual file remains gated behind forum credits/membership rather than exposed as a public direct download.

### Exact schematic independently indexed
RepairLap lists the exact attachment `Fujitsu_FH6C_FH6_hm70_r0c_mb_0522.pdf` together with 2 MB and 4 MB firmware images under Fujitsu LIFEBOOK AH532 / Quanta FH6 / DA0FH6MB6E0 Rev.E. It also separately lists U35, U7 and U24 firmware files. This independently corroborates the Vinafix package inventory.

Source:
- https://www.repairlap.com/threads/fujitsu-lifebook-ah532-schematic-da0fh6mb6e0-rev-e-bios.5293/post-8615

Vinafix exposes the exact PDF filename, board identity, controller summary and SHA-256, but marks download access as paid membership. The same site reports 1.4 MB size and 1,396+ views, indicating that the file is not merely a search-engine filename artifact.

Source:
- https://vinafix.com/threads/fujitsu-ah532-fh6-fh6c-hm70-r0c_mb_0522.15516/

### BoardView request trail
A dedicated DR-BIOS request exists specifically for `DA0FH6MB6E0 rev E`, and the administrator points to the FH6C/HM70 schematic/boardview package. This is independent evidence that technicians were looking for the boardview for the same exact revision.

Source:
- https://dr-bios.com/threads/da0fh6mb6e0.54744/

The Vinafix discussion is particularly useful because in 2018 a user explicitly requested the DA0FH6MB6E0 Rev.E boardview and the thread response was **"Unavailable bv"**. This explains why the boardview is repeatedly indexed today but rarely exposed as a directly downloadable public file.

Source:
- https://vinafix.com/threads/fujitsu-ah532-fh6-fh6c-hm70-r0c_mb_0522.15516/page-2

### AH530-Q / AH532 board-family clue
An independent electronics-repair forum has a schematic request explicitly naming `Fujitsu-Siemens LIFEBOOK AH530-Q — DA0FH6MB6E0 Rev.E`. This is important because it shows the same board marking was used/associated with another closely related Fujitsu model. Therefore the board marking is stronger evidence than a model-name search alone, but it also means we must not assume every AH532 and AH530-Q has identical component population.

Source:
- https://eletronicabr.com/en/topic/202177-fujitsu-ah530_q-da0fh6mb6e0-rev-e/

### Additional forum index
Another repair forum independently indexes `Fujitsu Lifebook AH532 Quanta FH6 DA0FH6MB6E0 DAFH6CMB6D0 REV:E schematic` and separately `Quanta FH6C DAFH6CMB6D0 schematic`. This suggests that DA0FH6MB6E0 and DAFH6CMB6D0 naming variants occur in the repair ecosystem and should be preserved as aliases during future searches.

Source:
- https://forum.diacom.az/viewforum.php?f=125

### Important outcome
The documentary search has now reached a practical boundary: the same primary/near-primary artifacts are independently indexed, but the actual schematic/boardview bytes are access-controlled or not exposed through searchable pages. The next useful move is therefore **not another generic web search**. It is either obtaining the actual archive through an accessible legitimate source or proceeding with machine-specific read-only hardware/ACPI evidence and later matching it against the board package.


## 29. Fifth-pass result: source boundary reached for the exact board package

The latest search checked the exact schematic filename, exact board ID, firmware filenames, and alternate archive names. It found no new publicly exposed copy of the actual PDF/BoardView bytes beyond the already indexed repair archives.

Important new evidence:
- RepairLap exposes the exact schematic attachment `Fujitsu_FH6C_FH6_hm70_r0c_mb_0522.pdf` (1.5 MB) together with 2 MB and 4 MB firmware images for DA0FH6MB6E0 Rev.E. It also separately exposes U35/U7/U24 images. Source: https://www.repairlap.com/threads/fujitsu-lifebook-ah532-schematic-da0fh6mb6e0-rev-e-bios.5293/post-8615
- Vinafix's technical-documentation index exposes the exact PDF's SHA-256 and confirms download access is membership-gated. Source: https://vinafix.com/forums/technical-documentation.15/page-35?direction=desc&order=reply_count
- AlexLaptopRepair independently indexes `FUJITSU LIFEBOOK A532AH532 DA0FH6MB6E0 rev E schematic.rar`, 633.8 KB, confirming another archive copy. Source: https://www.alexlaptoprepair.com/forums/threads/fujitsu-lifebook-a532-ah532-da0fh6mb6e0-rev-e-schematic.5056/
- DR-BIOS explicitly advertises a boardview entry for `fujitsu fh6c fh6 hm70 rev-0c mb 0522`. Source: https://dr-bios.com/threads/da0fh6mb6e0.54744/
- The Telegram archive lists `DA0FH6MB6E0 rev E PDF .rar` as both SCHEMATIC and BOARDVIEW, 679.4 KB. Source: https://t.me/s/schematicslaptop?before=10575

### Practical conclusion
The exact board package is real and repeatedly independently indexed. The public search layer has now reached the point where it returns archive/index metadata rather than the underlying file bytes. We should not claim to have inspected the schematic or boardview until the actual files are obtained through an accessible legitimate route.

For Aether engineering, the next highest-value action is now machine-specific read-only acquisition: identify the physical PCH by PCI ID, enumerate D20-D31, dump BARs/subsystem IDs/revisions, enumerate ACPI table identities, and collect EDID/USB topology. Those results can then be mapped against the known board-package fingerprints without unsafe MMIO or firmware operations.


## 30. Fifth-pass findings: G21/G52 and firmware lineage

A further search found useful evidence about AH532 product variants and firmware lineage.

### Fujitsu officially distinguishes G21 and G52
A Fujitsu service-package index lists separate BIOS update identifiers for LIFEBOOK AH532/G21 and AH532/G52, including FPC03500BK for G21 and FPC03465BK for G52. It also lists multiple AH532 generic BIOS package identifiers. This confirms that G21/G52 should not be treated as merely cosmetic labels when matching firmware or board evidence.

Source:
- https://www.fmworld.net/globalpc/batteryctrl/doc/FTS_april.pdf

### Fujitsu product data confirms HM76 and multiple graphics populations
A Fujitsu AH532/GFX data sheet explicitly identifies the chipset as Intel HM76 and lists G21/G52 base units with Intel HD Graphics 3000 or 4000 depending on CPU, plus discrete NVIDIA options. It also confirms Intel Centrino Wireless-N 2230 and the 4-in-1 card reader. This supports our rule that the exact CPU/graphics population must be taken from the physical machine rather than inferred from the model name.

Source:
- https://community.intel.com/cipcp26785/attachments/cipcp26785/wireless/16104/1/ds-LIFEBOOK-AH532GFX.pdf

### Independent G21/G52 firmware evidence
A BIOS-modification archive states that the same BIOS v2.09 family was used for AH532/G21 and AH532/G52, while a separate user report describes a G52 for which G21 BIOS files did not work. These are community reports, not authoritative Fujitsu compatibility documentation, but together they show that firmware matching deserves caution and that board/revision identity is more reliable than model suffix alone.

Sources:
- https://www.bios-mods.com/forum/Thread-FUJITSU-LIFEBOOK-AH532-G21
- https://forums.tomsguide.com/threads/fujitsu-laptop-motherboards-bios-problem.431705/post-1842700

### Exact board dump independently reported as read from a working AH532/G52
A firmware archive explicitly labels a dump as **Fujitsu LIFEBOOK AH532/G52**, main board `DA0FH6MB6E0 Rev.E`, and says it was read from a working machine. This is strong independent confirmation of the board/model relationship.

Source:
- https://remont-aud.net/dump/kompjutery_noutbuki_netbuki/fujitsu/433-4

### 4PDA evidence: exact files were circulated to AH532 owners
A long-running AH532 discussion independently references the exact schematic filename, a Google Drive copy, and separate EC/Main firmware archives for `DA0FH6MB6E0 REV-E`. This provides another trail to the same primary artifacts, although the forum post itself does not prove the contents of those archives.

Source:
- https://4pda.to/forum/index.php?showtopic=422156&st=220

### New engineering implication
We now have enough evidence to separate three layers cleanly:
1. Fujitsu product-family documentation (HM76, G21/G52, supported CPU/GPU/card-reader/WLAN populations);
2. exact board-family evidence (FH6/FH6C, DA0FH6MB6E0 Rev.E, schematic/firmware/boardview archives);
3. machine-specific truth, which Aether must obtain from PCI/ACPI/EDID/USB/runtime diagnostics.

This prevents the common mistake of taking a G21/G52 specification or a repair dump and treating it as the exact configuration of the user's AH532.

## 31. Sixth-pass board-package and physical-board cross-check

### Exact schematic family has a second independently indexed representation

A separate schematic catalogue identifies the A532/AH532 board as **Quanta FH6 / FH6C** and lists the PCB markings **DA0FH6MB6E0, DAFH6CMB6D0 and similar**. It states that the schematic is a 45-page PDF. This is useful because it connects the two board-name variants that repeatedly appear in repair archives instead of treating them as unrelated boards.

Source:
- https://realschematic.com/shop/9971/desc/fujitsu-lifebook-ah532-a532

A separate Fujitsu schematic index names the A532 board as **DAFH6CMB6D0 / Quanta FH6C** and describes the document as a schematic for that platform. This independently confirms that the DAFH6CMB6D0 spelling is a real board-family alias rather than a search typo.

Sources:
- https://www.gadget-manual.com/fujitsu/
- https://www.alifixit.com/fujitsu-lifebook-a532-dafh6cmb6d0-quanta-fh6c-schematic/

### DAFH6CMB6D0 has its own indexed schematic fingerprint

Vinafix's technical-documentation archive contains a separate **Fujitsu LifeBook A532 DAFH6CMB6D0 hm70 Quanta FH6C rB** document. It reports:
- board/platform: DAFH6CMB6D0 / A532;
- ODM: Quanta;
- DDR3;
- key controllers: Realtek ALC269 and RTL8111F;
- PDF size: 904.4 KB;
- SHA-256: `9e657015ab91c53a49dada2d245fc2e5ae85f28caabfc9c5676d03d269d01d72`.

This is important because it demonstrates that the FH6C family has multiple documented board/revision representations. The existence of the file does not prove that every DA0FH6MB6E0 Rev.E AH532 uses the exact same component population.

Source:
- https://vinafix.com/forums/technical-documentation.15/page-902?direction=asc&order=view_count

### Physical-board sales listings provide an additional HM76 ↔ DA0FH6MB6E0 correlation

Current Ukrainian parts listings independently identify working Fujitsu A532/AH532 UMA boards as:
- DA0FH6MB6E0;
- Rev.E on some listings;
- HM76;
- CP581562-01;
- suitable for third-generation Intel processors.

These are commercial listings rather than engineering documentation, so they are not treated as authoritative electrical evidence. They are nevertheless useful as an independent cross-check against the long-standing filename conflict where some community schematics are labelled "HM70".

Sources:
- https://prom.ua/ua/m-2484169030447441882-materinskaya-plata-fujitsu.html
- https://prom.ua/ua/m-604877864161845838-materinskaya-plata-fujitsu.html
- https://notebook-store.com.ua/ru/materynska-plata-fujitsu-lifebook-ah532-da0fh6mb6e0-hm76-harantiia/

### BoardView / maintenance-guide evidence has strengthened, but payload inspection is still pending

A recent repair-forum thread explicitly describes a **Fujitsu LifeBook A532 DAFH6CMB6D0 hm70 maintenance guide** containing a PDF and FZ boardview files. The forum text says the FZ files list components on the motherboard PCB. This is evidence that a boardview/maintenance package is circulating in usable form, but the forum's searchable page does not expose the actual archive bytes to us.

Source:
- https://www.diy-laptoprepair.com/forum/fix-FUJITSU-LifeBook-A532-DAFH6CMB6D0-hm70-repair-guide-schematics.html

The current Telegram archive also independently lists:
- `Fujitsu LifeBook A532 DAFH6CMB6D0 PDF .rar` — 872.8 KB;
- `Fujitsu LifeBook A532 DAFH6CMB6D0 hm70 Quanta FH6C rB PDF .rar` — 973.9 KB;
both explicitly tagged **#SCHEMATIC** and **#BOARDVIEW**.

Sources:
- https://t.me/s/schematicslaptop?before=11966
- https://t.me/s/schematicslaptop?before=11969

These findings materially improve the source map: we now have distinct archive trails for both DA0FH6MB6E0 Rev.E and DAFH6CMB6D0/RB variants.

### Exact-hardware matching rule remains unchanged

The new evidence increases confidence that the AH532/A532 platform uses the FH6/FH6C board family, but it does **not** replace machine-specific verification. Before using any schematic net name, EC address, GPIO, power rail, or boardview coordinate in Aether, require matching physical-board evidence:
1. board marking/revision;
2. PCH PCI ID/revision;
3. DMI/SMBIOS identity where available;
4. matching controller/subsystem IDs.

Until the actual schematic/boardview payload is inspected, all net-level information remains unverified.

## 32. Seventh-pass: a potentially accessible maintenance-guide lead

A new independent repair-forum report materially changes the status of the DAFH6CMB6D0 documentation lead. The forum states that a user obtained a **Fujitsu LifeBook A532 DAFH6CMB6D0 Quanta maintenance guide (PDF + FZ)** from a blog referenced through Reddit. The report specifically says the package contains the PDF plus **FZ boardview files listing components on the motherboard PCB** and that the files were downloadable after a survey. Multiple subsequent commenters state that they downloaded the same guide.

Source:
- https://www.diy-laptoprepair.com/forum/fix-Fujitsu-LifeBook-A532-DAFH6CMB6D0-Quanta-repair-guide-schematics.html

A closely related thread for the HM70-labelled variant reports the same result: the package contains a maintenance-guide PDF and FZ boardview files, and commenters confirm successful download.

Source:
- https://www.diy-laptoprepair.com/forum/fix-FUJITSU-LifeBook-A532-DAFH6CMB6D0-hm70-repair-guide-schematics.html

### What this proves — and what it does not

This is stronger than merely finding another filename in an archive index:
- the existence of a **PDF + FZ boardview package** is independently reported by users who claim to have downloaded it;
- the boardview format is explicitly identified as FZ;
- the boardview is described as containing component listings for the PCB.

However, the search result does **not expose the actual PDF/FZ bytes or the originating blog URL**. Therefore Aether still must not claim to have inspected the boardview contents.

### New acquisition target

The exact package to locate is now:

`Fujitsu LifeBook A532 DAFH6CMB6D0 Quanta maintenance guide (pdf & fz)`

and the closely related:

`Fujitsu LifeBook A532 DAFH6CMB6D0 hm70 maintenance guide (pdf & fz)`

This should be searched by exact title, not merely by `DA0FH6MB6E0`, because it may expose a freely accessible copy of the actual boardview payload.

### Evidence hierarchy update

For future use, classify the board-level material as:

1. **Payload inspected by Aether team** — highest confidence for net/component claims.
2. **Actual downloadable payload identified but not yet inspected** — existence confirmed, contents pending.
3. **Independent user report of successful download** — strong availability evidence, but contents still unverified.
4. **Archive/index listing only** — confirms that a file/package is known to exist.
5. **Search-engine filename/third-party assertion only** — lead, not hardware evidence.

The DAFH6CMB6D0 PDF+FZ package is currently category 3, not category 1.

### Safety boundary

Even if the boardview is obtained, it must first be treated as documentation. Net names, EC addresses, GPIOs, power rails and component designators must be cross-checked against the physical board and runtime PCI/ACPI evidence before Aether uses them to drive hardware.
