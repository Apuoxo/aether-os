#!/usr/bin/env bash
set -euo pipefail

ISO="${1:?usage: .kemu/run.sh path/to/aether.iso}"
LOG="${KEMU_LOG:-.kemu/logs/boot.log}"
QGA="${KEMU_QGA:-.kemu/logs/qga.sock}"

mkdir -p "$(dirname "$LOG")"
rm -f "$LOG" "$QGA"

timeout 90s qemu-system-x86_64 \
  -machine q35,accel=tcg \
  -m 2048 -smp 2 \
  -vga virtio \
  -display none \
  -chardev socket,id=qga,path="$QGA",server=on,wait=off \
  -device virtio-serial \
  -device virtserialport,chardev=qga,name=org.qemu.guest_agent.0 \
  -serial "file:$LOG" \
  -cdrom "$ISO" \
  -boot d \
  -no-reboot -no-shutdown || test $? -eq 124

grep -q 'AETHER_AI_BRIDGE=READY' "$LOG"
grep -q 'AETHER_GUI=READY' "$LOG"

python3 - "$QGA" <<'PY'
import json, socket, sys, time

path = sys.argv[1]
deadline = time.time() + 20
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
while True:
    try:
        s.connect(path)
        break
    except OSError:
        if time.time() >= deadline:
            raise SystemExit("KEMU: QGA connection timeout")
        time.sleep(0.25)

def cmd(obj):
    s.sendall((json.dumps(obj) + "\r\n").encode())
    buf = b""
    while b"\r\n" not in buf:
        chunk = s.recv(4096)
        if not chunk:
            raise SystemExit("KEMU: QGA disconnected")
        buf += chunk
    return json.loads(buf.split(b"\r\n", 1)[0])

cmd({"execute": "guest-sync-delimited", "arguments": {"id": 1}})
r = cmd({"execute": "guest-exec", "arguments": {
    "path": "/bin/sh",
    "arg": ["-c",
      "systemctl is-active --quiet aether-ai.service && "
      "systemctl is-active --quiet lightdm.service && "
      "test -S /tmp/.X11-unix/X0 && "
      "grep -q 'AETHER_AI_BRIDGE=READY' /run/aether/state && "
      "grep -q 'AETHER_GUI=READY' /run/aether/state"],
    "capture-output": True
}})
pid = r["return"]["pid"]
deadline = time.time() + 15
while time.time() < deadline:
    r = cmd({"execute": "guest-exec-status", "arguments": {"pid": pid}})
    if r.get("return", {}).get("exited"):
        code = r["return"].get("exitcode", 255)
        if code != 0:
            raise SystemExit(f"KEMU: guest readiness check failed (exit={code})")
        print("KEMU: guest state independently verified")
        raise SystemExit(0)
    time.sleep(0.25)

raise SystemExit("KEMU: guest readiness check timeout")
PY

echo "KEMU: PASS"
