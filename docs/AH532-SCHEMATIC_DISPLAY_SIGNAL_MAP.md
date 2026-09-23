# AH532 / FH6C schematic display signal map

Last reviewed: 2026-09-23

## Scope

This document records display-related physical routing extracted from the verified 45-page FH6/FH6C HM70 schematic in the repository.

It is a board-family/variant reference. It does not override runtime hardware detection.

## Authoritative runtime GPU fact

The tested AH532 reports Intel Sandy Bridge HD Graphics 3000, PCI 8086:0116, Gen6.

The schematic contains Ivy Bridge and optional N13P variants, so those depicted components are not assumed to be installed in the tested machine.

## Internal LCD / LVDS

PCH-side signals documented on the display sheet include:

- LCD_EDIDCLK
- LCD_EDIDDATA
- LCD_TXLCLKOUT+/-
- LCD_TXLOUT0+/-
- LCD_TXLOUT1+/-
- LCD_TXLOUT2+/-
- LVDS_DIGON
- LVDS_PWM
- LCD_BLON_I

LCD connector:
- CN33
- ACES_50238-04071-001_LVDS

Panel/control circuitry:
- LCDVCC
- LCD_BK_POWER
- LVDS_DIGON_R
- LVDS_PWM_EC
- LCD power switch
- F6 3A/32V fuse

EDID:
- LCD_EDIDCLK / LCD_EDIDDATA
- R326/R327 = 2.2K pull-ups to 3V_S0 at the LCD side

## HDMI / DDI-B

PCH DDI-B signals documented:

- DDPB_AUXN/P
- DDPB_HPD
- DDPB_0N/P
- DDPB_1N/P
- DDPB_2N/P
- DDPB_3N/P

The HDMI connector is CN6.

The schematic maps the DDI-B data pairs through the HDMI TMDS circuitry and documents:
- HDMI_DDCCLK
- HDM_DDCDATA
- DDC5V
- HDMI_CON_HP
- Port-B_HPD
- INT_HDMI_SCL
- INT_HDMI_SDA

DDC level shifting and TMDS termination are explicitly shown.

## Other DDI paths

The schematic documents:
- DDPC_AUXN/P
- DDPC_HPD
- DDPC_0..3N/P
- DDPC_CTRLCLK
- DDPC_CTRLDATA
- DDPD_AUXN/P
- DDPD_HPD
- DDPD_0..3N/P
- DDPD_CTRLCLK
- DDPD_CTRLDATA

Exact runtime connector usage is UNKNOWN until correlated with actual AH532 detection.

## FDI

The documented CPU↔PCH transport includes:

- FDI_TXP/N0..7
- FDI_RXP/N0..7
- FDI_FSYNC0/1
- FDI_LSYNC0/1
- FDI_INT

These belong to the documented platform variant and should not be confused with the HD3000 register model.

## Driver implications

1. The physical board has documented LCD EDID and HDMI DDC paths.
2. This supports continuing with read-only GMBUS/EDID probing.
3. Aether must discover the actual GMBUS pin/connector relationship at runtime rather than assume it from signal names.
4. HPD should be treated as corroborating evidence, not as proof of connector identity by itself.
5. No speculative LVDS/HDMI power sequencing should be introduced during EDID validation.
6. GGTT/GSM physical region 0xDF800000 remains out of scope because it previously caused an AH532 reboot.

## Evidence status

| Item | Status |
|---|---|
| Verified FH6C/HM70 schematic | DOC |
| LCD LVDS physical routing | DOC |
| LCD EDID nets | DOC |
| HDMI DDI-B/TMDS routing | DOC |
| HDMI DDC/HPD circuitry | DOC |
| Exact runtime GMBUS pin mapping | UNKNOWN |
| Actual AH532 EDID read success | UNKNOWN until hardware test |
| Actual connector policy | UNKNOWN |
| Sandy Bridge HD3000 runtime identity | AETHER |

