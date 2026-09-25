# Aether OS Wi-Fi bring-up log

## 2026-09-25 — commit 7416d12018af8d449185c3b53498007a44686c96

Commit: `wifi: expand DVM command TFD ring and scheduler init`

AH532 real-hardware test:
- Intel WLAN 8:0.0, VID:DID 8086:0887, SUB=4062
- BAR0=F0D00000, MMIO=MAPPED
- Firmware loaded: VER=12A80601, INST=175172, DATA=81920
- CMDQ=READY
- SCAN24=SUBMITTED
- RX-RING=READY
- IRQ_COUNT=2
- ALIVE=SEEN, VALID=1, SUBTYPE=1
- Firmware execution STARTED
- Scan notifications 0x82/0x83/0x84 not observed
- No SSID/BSSID/channel results reported

Conclusion: firmware/ALIVE and command-queue initialization remain operational on AH532. Scan submission occurs, but scan response/notification delivery is not yet proven.

Next stage: expose DVM scheduler/command transport state after scan submission without changing the working firmware/ALIVE path.
