# Aether OS — Wi-Fi / Intel 2230

## Current status

**Bring-up: firmware and command/TX transport are partially working; scan/networking are not complete.**

Target hardware on the tested AH532:

| Field | Value | Status |
|---|---|---|
| Adapter | Intel Centrino Wireless-N 2230 | AH532 runtime target |
| PCI | 8086:0887 | verified |
| Subsystem | 8086:4062 | verified |
| BDF | 08:00.0 | verified in recorded AH532 tests |
| BAR0 | F0D00000 | verified in recorded AH532 tests |
| Firmware | iwlwifi-2030 family | bring-up |
| ALIVE | seen / valid | verified in recorded AH532 tests |
| Command queue | initialized | verified |
| RX ring | ready | verified |
| TX/SCD consumption | not proven | blocker |
| Scan results | not proven | incomplete |
| Association/IP networking | not implemented to completion | incomplete |

## What the current driver actually proves

Recorded AH532 tests established:

- PCI discovery and BAR0 MMIO mapping;
- firmware loading with observed firmware version `12A80601`;
- firmware ALIVE state;
- command-queue initialization;
- RX-ring setup;
- scan submission;
- interrupt activity.

They did **not** establish successful scan-result delivery or working networking.

The latest transport investigation focuses on the DVM scheduler/SCD/TFD ownership path. Do not replace the working firmware/ALIVE path with a broad driver rewrite.

## Current blocker

The key unresolved transport question is whether the hardware consumes the submitted TFD through the scheduler and advances the corresponding SCD state.

Required evidence before declaring TX/scan functional:

1. command TFD consumption;
2. SCD read/consumer advancement;
3. expected firmware command completion/notification;
4. scan notification and/or actual SSID/BSSID/channel results.

## Next controlled stage

Continue from the existing DVM transport state:

1. verify the dedicated byte-count (BC) table is allocated and programmed separately from the TFD ring;
2. populate the command queue BC entry using the correct legacy DVM format;
3. log BC state together with SCD/CBBC/TCSR state;
4. keep firmware loading, ALIVE handling and RX setup unchanged;
5. rebuild and test on AH532;
6. only after transport consumption is proven, advance to scan-result handling.

## Safety

No arbitrary MMIO writes, speculative firmware, automatic association, or disk operations should be introduced merely to diagnose Wi-Fi. Hardware-dependent claims must be backed by an AH532 result.

## Source references

- `kernel/src/drivers/wifi.rs`
- `docs/WIFI_BRINGUP_LOG.md`
- `docs/STATUS.md`

The older description of this file as only a PCI/MMIO preparation stage was superseded by the later firmware/ALIVE/TX transport bring-up.
