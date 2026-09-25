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

## 2026-09-25 — commit 4fe45d41d66c68b8f91191693cb5b3f86afa92dd

Commit: `wifi: recreate WF transport diagnostics in desktop terminal`

AH532 real-hardware test:
- Same Intel 2230 identity: 8:0.0, VID:DID 8086:0887, SUB=4062
- BAR0=F0D00000, MMIO=MAPPED
- Firmware loaded: VER=12A80601, INST=175172, DATA=81920
- CMDQ=READY
- SCAN24=SUBMITTED
- RX-RING=READY
- IRQ_COUNT=2
- ALIVE=SEEN, VALID=1, SUBTYPE=1
- Firmware execution STARTED
- Desktop terminal now exposed the transport snapshot:
  - SCD_STATUS=00000090
  - SCD_DRAM=00261100
  - CBBC=00002611
  - TCSR_CFG=8086
  - TCSR_STS=00000400
  - HBUS_WRPTR=00000001
- No scan notifications 0x82/0x83/0x84 observed
- No SSID/BSSID/channel results reported
- The displayed line also contained a literal `\\N`, showing a logging-format bug in the desktop transport line.

Conclusion: the new desktop logging worked and confirmed that the DVM command transport was publishing a command, but the observed TCSR configuration value was inconsistent with the intended command FIFO selection. The literal `\\N` was also a formatting defect. Firmware/ALIVE remained intact.

## 2026-09-25 — commit 298588df84e2f1bc829615ef179dfaa9e5345299

Commit: `wifi: correct DVM command FIFO transport configuration`

Code-side correction based on the previous AH532 transport evidence:
- TCSR command-channel address is now derived from `IWL_CMD_FIFO_NUM` (FIFO 7), instead of incorrectly using queue number 4.
- Queue status activation now includes `SCD_QUEUE_STATUS_MASK`.
- Desktop transport logging now emits a real newline instead of the literal `\\N`.
- Firmware loading, ALIVE handling, RX ring setup, and the polling/no-STI safety model were intentionally left unchanged.

Build/test status at log entry:
- Commit pushed to `main`.
- GitHub Actions build had started, but no successful ISO artifact had yet been verified at the time of this log update.
- AH532 hardware result for this correction is not yet available.

Next test:
- Wait for a green build and test the resulting ISO on AH532.
- Compare the new `SCD_STATUS`, `SCD_DRAM`, `CBBC`, `TCSR_CFG`, `TCSR_STS`, and `HBUS_WRPTR` values against the previous run.
- Do not claim native scan success until 0x82/0x83/0x84 notifications or actual SSID/BSSID/channel results are observed.
