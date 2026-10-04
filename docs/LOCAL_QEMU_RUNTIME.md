# Aether Local QEMU Runtime

## Purpose

QEMU is **not a user-facing Aether feature**. It is an offline instrument for me/the Aether runtime to create a disposable virtual Aether machine, run tests, observe the result, and reset or destroy the test machine.

The requirement is therefore much smaller than a general desktop virtualization environment.

## Two independent QEMU layers

### 1. CI QEMU

```
GitHub Actions
  -> build ISO
  -> QEMU
  -> serial/events
  -> diagnostic artifact
```

This is external validation. It does not validate AH532-specific Intel Gen6 graphics, HDA, or Intel 2230 WLAN behavior.

### 2. Local/offline QEMU

```
offline Aether medium
  -> local runtime/controller
  -> headless QEMU
  -> disposable test-Aether image
  -> serial/events/state
  -> local runtime/controller
```

There is no requirement for a QEMU desktop, VGA output, audio, keyboard, mouse, user interaction, or network access.

The minimum useful virtual machine needs only the devices and interfaces required to boot Aether and return machine state/diagnostics to the controller. The first target should prefer:

- serial console/log output;
- virtual CPU and RAM;
- virtual boot disk/CD image;
- deterministic reset/stop;
- machine exit status;
- no graphical display;
- no emulated audio;
- no user input devices unless a later test explicitly requires them;
- no virtual network unless a later test explicitly requires it.

## Important implementation boundary

The current Aether kernel is not a general POSIX host environment and cannot simply execute a normal desktop QEMU binary because it is present on the medium.

Therefore the implementation must first determine the **smallest headless QEMU runtime that can actually be hosted by Aether**. The target is not “QEMU with fewer UI features” in the abstract; it is a purpose-built offline test instrument with only the execution, memory, storage, serial, reset, and observation capabilities that Aether needs.

The runtime should be treated as a tool with a narrow controller interface:

```
CREATE_TEST_GUEST
  -> LOAD_IMAGE
  -> START
  -> OBSERVE
  -> STOP / RESET
  -> COLLECT_RESULT
```

The controller, not the user, owns this lifecycle.

## Safety boundary

The virtual guest is disposable. Its state must not silently become production Aether state.

QEMU is not identity and is not part of the reasoning model. It is an instrument used by the runtime to obtain additional observations and verification results.

## Next implementation sequence

1. Determine the minimum host execution environment Aether needs to launch the headless QEMU instrument.
2. Determine the minimum QEMU configuration and dependencies required for booting a disposable Aether guest.
3. Define the controller ABI for start/observe/stop/result.
4. Establish offline packaging on the boot medium.
5. Run the first completely headless guest and return only machine-readable diagnostics.
6. Connect those diagnostics to the existing observation/reasoning loop.

Until then, CI QEMU remains external validation and the real AH532 remains authoritative for hardware-specific behavior.
