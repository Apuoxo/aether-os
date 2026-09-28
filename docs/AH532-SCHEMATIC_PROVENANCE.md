# AH532 schematic provenance and extraction audit

Last reviewed: 2026-09-23

## 1. Verified target

The correct schematic family for the target machine is independently identified as:
- Fujitsu LIFEBOOK AH532 / A532
- Quanta FH6 / FH6C
- motherboard marking DA0FH6MB6E0
- Rev E
- schematic: `Fujitsu_FH6C_FH6_hm70_r0c_mb_0522.pdf`

The repository now contains a copy obtained from SKCLAPPY and verified by GitHub Actions.

## 2. Cryptographic verification

Repository file:
`Fujitsu_FH6C_FH6_hm70_r0c_mb_0522 SKCLAPPY.IN.pdf`

Verified:
- Size: 1,498,316 bytes
- SHA-256: `8011b1775a403b5d62ff816c278412f62c2e3d7873b955f2721fa664528badbc`
- Page count: 45
- PDF text extraction succeeds
- Technical identifier extraction succeeds

The verification workflow is:
`.github/workflows/verify-ah532-schematic.yml`

## 3. What the verified document actually contains

The 45-page document is a genuine FH6C/HM70-family schematic, but its variant content matters.

It explicitly includes:
- Ivy Bridge processor terminology;
- HM70 Panther Point;
- FH6 UMA consumer/commercial variants;
- FH6 N13P-LP and N13P-GLP discrete-GPU variants;
- FH6C project-disable/variant references.

Therefore it is a **verified board-family/variant schematic and physical signal-routing source**, not proof that every depicted processor/GPU option exists in the tested AH532.

## 4. Runtime-vs-schematic rule

The tested AH532 reports:
- PCI GPU 8086:0116;
- Intel Sandy Bridge HD Graphics 3000;
- Gen6.

That runtime evidence remains authoritative for the actual GPU.

The schematic's Ivy Bridge eDP signals and N13P blocks must not be promoted to installed-hardware facts without additional runtime or board-specific evidence.

## 5. Display routing extracted from pages 8, 24 and 25

### PCH / display transport
Page 8 documents FDI and PCH display paths, including:
- FDI_TXP/N0..7;
- FDI_RXP/N0..7;
- FDI_FSYNC0/1;
- FDI_LSYNC0/1;
- LCD/LVDS-related nets;
- DDI-B/C/D auxiliary, HPD and data paths.

### HDMI
Page 24 documents:
- connector CN6;
- TMDS pairs;
- HDMI_DDCCLK;
- HDM_DDCDATA;
- DDC5V;
- HDMI_CON_HP / Port-B_HPD;
- INT_HDMI_SCL/SDA;
- DDC level-shifting and termination circuitry.

### Internal LCD/LVDS
Page 25 documents:
- LCD connector CN33;
- LCD_TXLOUT0/1/2 and clock differential pairs;
- LCD_EDIDCLK/LCD_EDIDDATA;
- LVDS_PWM / LVDS_PWM_EC;
- LCD_BLON_I;
- LVDS_DIGON;
- LCDVCC and LCD_BK_POWER;
- LCD power-switch circuitry;
- EDID pull-ups R326/R327 = 2.2K to 3V_S0.

These are valuable for physical routing and connector investigation.

## 6. Other hardware blocks inspected

The 45 pages were read as a complete document. The schematic also contains:
- PCH PCIe/SMBus/LAN/WLAN-related sections;
- SATA/RTC/HDA/LPC;
- EC/power-management and GPIO-related sections;
- CPU/PCH power rails;
- discrete-GPU power circuitry.

Because some of these sections are variant-dependent, they should be used as physical reference only until correlated with runtime hardware.

## 7. Previous wrong document

The earlier 34-page PDF from `DA0FH6MB6E0 rev E PDF .rar` is internally identified as:
- FH2;
- Arrandale;
- HM55;
- Ibex Peak-M / Calpella-era platform.

It is therefore permanently classified REFERENCE-MISMATCH for AH532 development.

## 8. Documentation consequence

The repository now has three distinct evidence classes:
1. **Runtime facts** from the actual AH532.
2. **Verified FH6/FH6C schematic facts** for board-family physical routing.
3. **Rejected historical/reference material** that does not match the target.

This distinction must be preserved in future driver work.

## 9. Next documentation/engineering stage

Use the verified schematic together with Gen6 runtime/register evidence to build and validate the display connector path:
1. correlate GMBUS pins with the documented LCD/HDMI DDC paths;
2. test EDID read-only;
3. correlate HPD where available;
4. document results as AETHER runtime evidence;
5. only then consider connector-specific policy or modeset changes.
