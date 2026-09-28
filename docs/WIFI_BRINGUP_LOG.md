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

 
## 2026-09-25 — transport correction: SCD byte-count table
 
AH532 result for commit `298588df84e2f1bc829615ef179dfaa9e5345299`:
- SCD_STATUS changed from `00000090` to `0000009F`.
- TCSR_CFG changed from `00000806`-style invalid output to `80008008`; the observed value now contains DMA enable, host-end-TFD completion, and credit-enable bits.
- SCD_STATUS `0x9F` is consistent with the command queue being active, mapped to FIFO 7, with the scheduler status mask applied.
- HBUS_WRPTR remains `00000001`.
- No `0x82/0x83/0x84` scan notifications and no SSID/BSSID/channel results were observed.
 
### Root-cause finding before next code change
 
The transport snapshot exposed a more important remaining defect in the implementation: `SCD_DRAM_BASE_ADDR` was being programmed with the **TFD ring address**. In Intel's gen1/2 iwlwifi transport, this register points to the scheduler **byte-count table (BC table)**, while the TFD circular-buffer base is supplied separately through the FH CBBC queue register. The kernel implementation allocates the BC tables separately and programs `SCD_DRAM_BASE_ADDR` from that DMA address. citeturn5search0turn6search7
 
For the legacy DVM scheduler, the BC table is 320 u16 entries per queue (256 normal entries plus 64 duplicate entries). The command queue is #4, so the hardware must index queue #4 within a table covering the queue set; the command entry must be populated with its transfer length in DWORDs on this pre-AX210 transport. citeturn11search0turn13search0
 
### Next commit
 
The next code commit will therefore:
1. Allocate a dedicated DMA-visible BC table for the legacy scheduler.
2. Program `SCD_DRAM_BASE_ADDR` from the BC-table base instead of the TFD ring.
3. Populate queue #4's BC entry for each submitted command, including the required duplicate entry for the first 64 slots.
4. Log `CMD_BC_DW` alongside the existing transport registers.
5. Leave firmware loading, ALIVE, RX ring, no-STI polling, TCSR FIFO selection, and scan payload unchanged.
 
This is still a transport-stage fix. Native scan success will only be declared after the firmware produces the documented scan response/notifications or actual scan results. citeturn3view0turn1search2
