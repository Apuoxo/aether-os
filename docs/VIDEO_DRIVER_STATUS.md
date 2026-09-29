# Intel Gen6 Video Driver Status

**Last reviewed: 2026-09-29**  
**Target: Fujitsu LIFEBOOK AH532, runtime-detected Intel Sandy Bridge HD Graphics 3000, PCI 8086:0116.**

## Current verified state

- Multiboot framebuffer desktop path is working on AH532.
- KMS/modeset has been experimentally observed.
- Intel Gen6 MMIO/register diagnostics are available.
- The 1366x768 desktop path exists; mouse geometry is based on current framebuffer dimensions.
- The current desktop uses the stable framebuffer presentation path.
- GGTT/GSM access around physical 0xDF800000 remains disabled because previous experiments caused an AH532 reboot.
- EDID/GMBUS probing is implemented as an explicit diagnostic path, but real-AH532 EDID success is still unproven.

## Current source implementation

- Intel PCI display-controller discovery;
- BAR0 decoding and guarded MMIO mapping;
- read-only display-engine snapshots;
- primary-plane/scanout inspection;
- GMBUS/EDID transaction path;
- framebuffer geometry validation;
- desktop/window rendering through the existing framebuffer;
- diagnostic video state commands.

## Not complete

- independent cold-boot native Intel modeset without relying on firmware/Multiboot scanout;
- safe GGTT/GSM initialization;
- hardware-accelerated rendering;
- complete connector/power management;
- verified real-AH532 EDID acquisition;
- production-grade Intel display driver;
- suspend/resume graphics state management.

## Safety boundary

Do not re-enable access to physical 0xDF800000 merely because a QEMU experiment succeeds.

Do not treat KMS=READY as proof of a complete Intel graphics driver.

Do not replace the stable framebuffer presentation path with a partial backbuffer or memory-management experiment.

## Verified board-family reference

The repository contains a verified 45-page FH6/FH6C HM70-family schematic. It is a board-family/variant signal-routing reference, not proof of the exact CPU/GPU population of the tested AH532. Runtime PCI identification remains authoritative.

## Next controlled graphics stage

1. run explicit read-only EDID/GMBUS probing on AH532;
2. record successful 128-byte EDID/checksum/timing evidence if obtained;
3. correlate with the board-family signal map without assuming connector identity;
4. keep framebuffer ownership unchanged;
5. only then consider native display policy and graphics-memory work.
