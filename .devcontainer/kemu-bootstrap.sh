#!/usr/bin/env bash
set -euo pipefail

echo "KEMU: LAB VM READY"
echo "KEMU: QEMU=$(qemu-system-x86_64 --version | head -n1)"

sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  build-essential \
  bc bison flex cpio debhelper debootstrap grub-pc-bin grub-efi-amd64-bin \
  grub-common xorriso squashfs-tools busybox curl ca-certificates \
  qemu-system-x86 qemu-utils

mkdir -p .kemu/logs .kemu/artifacts
cat > .kemu/run.sh <<'RUN'
#!/usr/bin/env bash
set -euo pipefail
ISO="${1:?usage: .kemu/run.sh path/to/aether.iso}"
LOG=".kemu/logs/boot.log"
rm -f "$LOG"
timeout 90s qemu-system-x86_64 \
  -machine q35,accel=tcg \
  -m 2048 -smp 2 \
  -display none \
  -serial "file:$LOG" \
  -cdrom "$ISO" \
  -boot d \
  -no-reboot -no-shutdown || true

grep -q 'AETHER_GUI=READY' "$LOG"
grep -q 'AETHER_AI_BRIDGE=READY' "$LOG"
echo "KEMU: PASS"
RUN
chmod +x .kemu/run.sh
echo "KEMU: BOOT HARNESS INSTALLED"
