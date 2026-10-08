#!/usr/bin/env bash
set -euo pipefail

ISO="${1:?usage: .kemu/run.sh path/to/aether.iso}"
LOG="${KEMU_LOG:-.kemu/logs/boot.log}"

mkdir -p "$(dirname "$LOG")"
rm -f "$LOG"

timeout 90s qemu-system-x86_64 \
  -machine q35,accel=tcg \
  -m 2048 -smp 2 \
  -vga virtio \
  -display none \
  -serial "file:$LOG" \
  -cdrom "$ISO" \
  -boot d \
  -no-reboot -no-shutdown || test $? -eq 124

grep -q 'AETHER_GUI=READY' "$LOG"
grep -q 'AETHER_AI_BRIDGE=READY' "$LOG"

echo "KEMU: PASS"
