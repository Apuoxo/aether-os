# First external application launch — Linux ELF vertical slice

## Decision

The first external application Aether must execute is a real, independently built Linux x86_64 ELF, not an application written specifically for Aether.

The first acceptance binary is GNU Hello (or an equivalent unmodified GNU Hello release binary supplied for the test). It must be placed on a mounted hard-disk filesystem and loaded from that filesystem at runtime.

## Native Aether vs external ELF

ELF itself does not universally encode Linux versus Aether. Therefore the Runtime Manager uses launch provenance:

- a trusted Aether-native launch path may explicitly select NativeAether;
- external RUN <path> without an explicit personality resolves a compatible x86_64 ELF to Linux Personality;
- PE resolves to Windows Personality;
- APK/Android container resolution is reserved for the Android loader and must validate the package manifest before execution.

This prevents Aether bootstrap ELF files from silently being treated as Linux applications.

## Runtime path

    Aether Terminal (native)
            |
            v
       RUN <disk path>
            |
            v
       Aether VFS read
            |
            v
       Runtime Manager
            |
            +-- executable format
            +-- Personality resolution
            |
            v
       Linux Personality
            |
            v
       ELF loader
            |
            v
       Aether Process
            |
            v
       Linux ABI/syscall layer
            |
            v
       Aether kernel services

The terminal itself never changes Personality. Only the launched process receives the selected Personality.

## First Linux ABI gate

1. ELF64 x86_64 loading.
2. User address space creation.
3. Correct segment permissions.
4. Initial user stack.
5. Linux write.
6. Linux exit.
7. Process cleanup and Personality reference release.

No authentication prompt, VM, foreign Linux kernel, or Linux boot sequence is inserted before launch.

## Next gates

1. independently built static Linux CLI program using file I/O;
2. dynamic ELF with a controlled loader/runtime;
3. threads and futex;
4. sockets/networking;
5. GUI Linux application;
6. large real application such as Firefox.

Each gate must be a real external application and must not receive Aether-specific syscall patches.

## Current implementation boundary

The Runtime Manager currently provides executable-format resolution. It does not claim to launch Linux binaries yet. The next implementation commits must connect:

RUN path -> VFS read -> Runtime Manager -> ELF loader -> Process(personality=Linux)
