# Aether OS — Engineering Audit Baseline & Repair Plan

Baseline: 2026-09-23
Source of truth: current repository source + documented runtime evidence.
Purpose: prevent repeated rediscovery of already-audited architectural/code defects.

This document is the persistent audit baseline. Future development must use it as the starting point and update findings when they are fixed or disproven.

## P0 — must fix before architectural expansion

- [ ] Syscall ABI mismatch: isr.s, handlers.rs, and Ring3 programs disagree on argument registers/frame offsets. Define one ABI and add a regression smoke test.
- [ ] 64-bit TSS descriptor: replace the current incorrect two-entry construction with an explicit 16-byte x86_64 TSS descriptor and verify CPL3 to CPL0 entry.
- [ ] Process to Personality ownership: Process currently has no real Personality association; count_by_personality() does not actually filter ownership.
- [ ] Capability enforcement: capability structures exist, but resource/syscall access is not actually capability-gated. Implement object handles, rights checks and stale-handle protection before claiming capability security is implemented.

## P1 — kernel correctness / safety

- [ ] Harden ELF loader: checked arithmetic, segment bounds, canonical user VA checks, overlap checks, PT_LOAD permissions.
- [ ] Fix ELF page ownership accounting; LoadedImage.pages cannot represent all allocated pages.
- [ ] Replace fixed user_ok() range checks with validation against the current process address space and required access rights.
- [ ] Turn scheduler scaffolding into an actual timer-driven scheduler/context-switch path.
- [ ] Separate RAM AetherFS from persistent disk-writing code. During hardware validation, no normal boot path may write host disks.
- [ ] Define one AetherFS on-disk layout; remove conflicting/obsolete parsing formats.
- [ ] Document/enforce current AHCI 32-bit DMA limitation.
- [ ] Replace fixed 64 MiB PMM initialization with Multiboot memory-map based allocation.
- [ ] Make framebuffer mapping report/handle mapping failures explicitly.
- [ ] Replace fixed PS/2 mouse 799x599 bounds with current display geometry.
- [ ] Continue Intel HD 3000 Gen6 driver conservatively: no GGTT/GSM mapping until independently justified and tested.

## P2 — architecture completion

- [ ] Replace Linux/Android/Windows personality stubs with the real personality/module architecture.
- [ ] Implement actual personality lifecycle and unload semantics.
- [ ] Build the capability-mediated resource model before expanding compatibility personalities.
- [ ] Establish real scheduler/IPC/domain primitives needed by the long-term polymorphic architecture.
- [ ] Keep native Intel display work staged; do not let GUI features substitute for the underlying driver milestones.

## Documentation contradictions already identified

1. Architecture describes a capability/polymorphic microvisor, while current code is still a monolithic native kernel with integrated drivers/desktop and stub personalities.
2. Everything through capabilities is not true of current syscalls.
3. Personality ownership is documented architecturally but absent from Process.
4. Personality unload is currently a state toggle, not actual module code/data unloading.
5. Disk-write code conflicts with the current hardware-validation safety policy.
6. README status is stale relative to docs/STATUS.md.
7. Current PS/2 source may retain the old fixed 800x600 clamp despite dynamic geometry work elsewhere.
8. QEMU CPL3/syscall smoke evidence must not be interpreted as proof of correct syscall argument semantics.

## Development rule

For every repair:
1. change source;
2. build;
3. run deterministic QEMU test where applicable;
4. generate explicit runtime evidence;
5. test AH532 when hardware behavior is involved;
6. update this document and docs/STATUS.md;
7. only then mark the item complete.

Do not remove a finding merely because the system boots. A finding is closed only when the relevant source invariant and runtime evidence agree.

## Audit report

Detailed findings and rationale are preserved in docs/AUDIT_2026-09-23.md.