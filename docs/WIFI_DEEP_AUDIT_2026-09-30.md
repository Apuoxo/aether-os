# Wi-Fi 2230 Deep Driver Audit — 2026-09-30

## Scope

Target hardware:
- Intel Centrino Wireless-N 2230
- PCI ID 8086:0887
- Subsystem 8086:4062
- Intel 2030 DVM firmware family

Current Aether baseline:
- main: dd366e5c44a842233f07c4126322b9e358322338
- driver: kernel/src/drivers/wifi.rs
- current driver size: 96,145 bytes / 2,095 lines / 94 functions
- current state is diagnostic-only for Wi-Fi transport; no successful native scan has been proven.

## Required workflow

1. Treat Linux iwlwifi DVM/2030 code and Intel-compatible firmware/transport definitions as the protocol reference.
2. Audit the entire current wifi.rs, not only the failing path.
3. Compare every hardware-visible register, structure, offset, field, size, DMA address and state transition against the reference.
4. Separate facts, verified matches, mismatches, unknowns and hardware-dependent behavior.
5. Do not change RXON/SCAN payloads while the command transport is unproven.
6. One hypothesis -> one minimal change -> green CI -> one AH532 test -> record result.
7. Keep the existing working PCI/reset/activation/firmware/ALIVE path intact.

## Current hardware evidence

Latest AH532 evidence before this audit:
- PCI detection succeeds.
- MMIO mapping succeeds.
- CSR reset succeeds.
- NIC activation succeeds.
- 2030 firmware DMA/load succeeds.
- firmware execution starts.
- ALIVE is seen, VALID=1, SUBTYPE=1.
- command queue reaches READY.
- RX IRQ is observed.
- SCAN command is submitted.
- scheduler write pointer advances, but scheduler read pointer does not consume the submitted command.
- observed transport state includes TXF=0, TCSR=0x80000000, RD unchanged and no scan-complete notification.

This proves the blocker is below scan-result semantics and above normal scan completion: the command transport must be established first.

## Reference facts verified during this audit

Linux maps 8087:0887 / subsystem 4062 to iwl2030_2bgn_cfg and the 2030 device family.

For legacy AGN/DVM scheduler:
- SCD base = PRPH + 0xA02C00
- SCD_DRAM_BASE_ADDR = SCD_BASE + 0x08
- SCD_TXFACT = SCD_BASE + 0x10
- SCD_QUEUECHAIN_SEL = SCD_BASE + 0xE8
- SCD_CHAINEXT_EN = SCD_BASE + 0x244
- SCD_EN_CTRL = SCD_BASE + 0x254
- queue status TXF bits = 0..2
- queue ACTIVE = bit 3
- queue WSL = bit 4
- queue scheduler-active-enable = bit 19
- queue status mask = 0x017F0000
- context queue offset = 0x600 + queue*8
- scheduler window size = 64
- frame limit = 64

Linux transport initialization:
- obtains scheduler base from SCD_SRAM_BASE_ADDR
- clears scheduler context memory
- programs SCD_DRAM_BASE_ADDR from a dedicated scheduler byte-count table DMA address
- disables SCD chain extension when the 2030 base parameters require the workaround
- enables the command TX queue
- activates TX DMA/FIFO channels
- enables DMA and credit on all TX channels
- updates FH TX chicken bits for SCD auto retry
- queue status activation uses TXF/FIFO, ACTIVE and WSL plus the scheduler status mask.

The 2030 base parameters explicitly require scd_chain_ext_wa=true.

The Linux PCI table explicitly maps 0x0887/0x4062 to iwl2030_2bgn_cfg.

## Current Aether implementation: verified matches

- SCD base and major scheduler register offsets match the legacy AGN definitions.
- Queue #4 is used as the command queue.
- FIFO 7 is selected for the command queue.
- Queue status field positions are correct.
- SCD status mask is correct.
- Window size and frame limit are 64.
- SCD byte-count table is now separate from the TFD ring.
- SCD_DRAM_BASE_ADDR is programmed from the BC table rather than the TFD ring.
- TFD host publication uses HBUS_TARG_WRPTR at MMIO + 0x460.
- TX channels are configured over channels 0..7.
- RXON field offsets currently used are consistent with the DVM iwl_rxon_cmd layout.
- Passive scan with no probe TX is a legitimate diagnostic choice; missing probe TX is not treated as the transport root cause.

## High-priority mismatches / unknowns

### 1. SCD_TXFACT activation is suspect

Aether currently writes:
SCD_TXFACT = 1 << 7

This enables only FIFO 7.

Linux transport initialization activates all scheduler TX FIFOs, using the equivalent of an 0xFF activation mask.

This is a concrete implementation difference. It is not yet declared the root cause because exact 2030 hardware behavior must be isolated experimentally.

### 2. SCD_EN_CTRL path is not implemented

Aether explicitly documents that command queue #4 is not enabled through SCD_EN_CTRL and does not write SCD_EN_CTRL.

Linux has a conditional scheduler-active path for transports/configurations that require it. Therefore SCD_EN_CTRL must be treated as a configuration-dependent branch, not blindly enabled.

The current diagnostic commit dd366e5 exists specifically to expose:
- direct SCD_EN_CTRL value
- queue status bit 19 (SCD_ACT)

The next hardware result must determine whether scheduler activation is actually missing.

### 3. FH command-channel state remains unexplained

The latest hardware result showed:
- TCSR = 0x80000000
- TXF = 0
- SCD WR advanced
- SCD RD did not advance

This pattern means the host published the command but the transport did not consume it. It is not sufficient to conclude that the TCSR value alone is wrong; the scheduler/FH relationship must be traced from the reference initialization sequence.

### 4. The diagnostic path still reads MMIO + 0x60 as HBUS_WRPTR

The actual command publication path correctly writes MMIO + 0x460.

The scan diagnostic still prints MMIO + 0x60 under the label HBUS_WRPTR. This is misleading telemetry and must be removed or relabeled in a future diagnostic cleanup. It is not currently used for command submission.

### 5. Current scan command transport cannot yet prove firmware semantic rejection

Because the scheduler does not consume the command, absence of REPLY_SCAN or scan notifications cannot be interpreted as a scan-payload defect.

## Lower-priority findings to audit after transport

- Exact DVM command-header/sequence semantics versus Aether TFD sequence construction.
- Exact TFD descriptor layout and byte-count semantics.
- Exact BC-table layout, duplicate-entry rules and queue indexing.
- Physical-address constraints/alignment for TFD, BC and firmware DMA buffers.
- Complete FH TCSR/TRB/TSSR register semantics for the 2000/2030 generation.
- MAC-access lifecycle around transport initialization.
- RX RBD/status-ring structure and producer/consumer semantics.
- Interrupt mask/ack/clear semantics.
- Firmware TLV parsing, section selection, alignment and execution start sequence.
- Firmware API 5/6 compatibility and exact embedded firmware identity.
- RXON semantic completeness after transport is proven.
- Required TX-power/configuration commands after RXON.
- NVM/MAC/radio calibration state before scan.
- Scan command field-by-field comparison after RXON response is proven.
- Recovery/reset behavior after transport timeout.

## Strategic execution plan

### Phase A — complete reference model
Build a local reference table for:
PCI -> reset -> MAC access -> firmware DMA -> ALIVE -> scheduler init -> command submission -> RX response -> RXON -> scan -> notifications.

For every stage record:
address, width, structure, size, legal values, prerequisite, side effect, expected readback and failure signature.

### Phase B — complete current-driver audit
Scan wifi.rs function-by-function and classify each hardware-visible operation:
VERIFIED / MATCH / MISMATCH / UNKNOWN / DIAGNOSTIC-ONLY / UNSAFE.

No behavior change in this phase.

### Phase C — isolate scheduler activation
Use the dd366e5 diagnostic result to determine:
- SCD_EN_CTRL
- SCD_ACT bit
- SCD_TXFACT
- queue status
- FH TCSR
- FH TSSR
- TRB
- WR/RD movement

Only then select the next single change.

### Phase D — repair the first proven transport mismatch
Candidate priority:
1. scheduler/FIFO activation if proven missing;
2. scheduler-active control if proven required;
3. FH command-channel configuration if proven inconsistent;
4. TFD/BC semantics if proven inconsistent.

No simultaneous changes.

### Phase E — prove command response
Require observable REPLY_RXON / REPLY_SCAN or REPLY_ERROR evidence before touching scan semantics.

### Phase F — prove scan
Only after command transport and RXON are proven:
- validate RXON semantic fields
- send required configuration commands
- compare scan command byte-for-byte with DVM reference
- require 0x82/0x83/0x84 or a documented error response.

### Phase G — productionize
After successful AH532 scan:
- reduce diagnostic noise
- retain structured counters
- document exact working sequence
- add recovery
- only then move to association/authentication/IP.

## Current decision

Do NOT change RXON or SCAN payloads now.

The immediate investigation target is the command scheduler/FIFO activation path. The strongest concrete implementation difference currently identified is SCD_TXFACT=0x80 versus the reference all-FIFO activation behavior. The SCD_EN_CTRL path is a second conditional hypothesis and must be resolved from the new diagnostic state rather than guessed.

This document is an audit/plan commit only; it does not change Wi-Fi hardware behavior.
