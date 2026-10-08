# KEMU — Aether external virtual-machine laboratory

KEMU is an external test layer. It is not part of Aether OS and is never copied into the ISO.

The intended flow is:

1. Open this branch in GitHub Codespaces.
2. Codespaces provides the remote development VM.
3. The dev container installs QEMU and the Aether image-build dependencies.
4. Build the ISO with `tools/build-boot-image.sh`.
5. Run `.kemu/run.sh out/aether-os-6.18.55-x86_64.iso`.
6. KEMU checks the serial boot contract for GUI and AI readiness.

This is deliberately the first minimal layer: environment + external emulator + deterministic PASS/FAIL.
