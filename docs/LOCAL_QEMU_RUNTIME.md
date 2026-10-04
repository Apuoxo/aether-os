# Aether Local QEMU Runtime

## Purpose

Aether must retain a completely offline test path. GitHub Actions QEMU is only the external CI laboratory; it cannot be the only QEMU available once Aether is booted without network access.

## Two independent QEMU layers

### 1. CI QEMU

```
GitHub Actions
  -> build ISO
  -> QEMU
  -> serial/events
  -> diagnostic artifact
```

This validates the boot image in a controlled external environment. It does not validate AH532-specific Intel Gen6 graphics, HDA, or Intel 2230 WLAN behavior.

### 2. Local/offline QEMU

The eventual offline environment is:

```
Aether boot media
  -> Aether runtime
  -> local test controller
  -> QEMU
  -> disposable test-Aether image
  -> serial/events/state
  -> Aether runtime
```

The QEMU executable and all required runtime data must be available from the offline medium. The test path must not depend on GitHub, downloads, or an external network.

## Current implementation boundary

The current Aether image is a native `no_std` kernel booted directly by GRUB. Its present ELF loader is a small x86_64 ET_EXEC loader with bounded images; it is not yet a general POSIX userspace capable of executing a large host application such as QEMU.

Therefore this requirement must **not** be implemented by merely copying a QEMU binary into the ISO. Such a binary would be inert until Aether has:

- a persistent executable filesystem accessible after boot;
- a sufficiently complete Ring3 process/runtime ABI;
- executable loading for the required QEMU binary and its dependencies;
- memory/address-space support adequate for QEMU;
- the device/timer/threading primitives required by the selected QEMU build;
- a safe controller interface for creating, observing, stopping, and collecting a test guest.

## Safety rule

Local QEMU is a testing instrument, not part of identity. It may create and control disposable test guests, but it must not bypass Aether's normal boot path or silently mutate the production runtime.

## Next implementation sequence

1. Establish an offline runtime filesystem/executable boundary.
2. Establish a minimal general Ring3 process/ABI boundary.
3. Define the local test-controller interface.
4. Select a reproducible QEMU build suitable for offline packaging.
5. Package QEMU plus required runtime assets on the boot medium.
6. Add a local controller that launches a disposable Aether test image.
7. Capture serial/events/state back into the running Aether runtime.
8. Only then enable autonomous self-test actions.

Until these prerequisites exist, CI QEMU remains external validation and the real AH532 remains authoritative for hardware-specific behavior.
