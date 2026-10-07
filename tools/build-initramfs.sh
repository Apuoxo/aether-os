#!/usr/bin/env bash
set -euo pipefail
ROOT="$PWD/build/rootfs"
rm -rf "$ROOT"
mkdir -p "$ROOT"/{bin,dev,etc,proc,sys,run,tmp,var}
cp /bin/busybox "$ROOT/bin/busybox"
ln -s busybox "$ROOT/bin/sh"
ln -s busybox "$ROOT/bin/mount"
ln -s busybox "$ROOT/bin/echo"
ln -s busybox "$ROOT/bin/uname"
ln -s busybox "$ROOT/bin/ls"
ln -s busybox "$ROOT/bin/cat"
ln -s busybox "$ROOT/bin/ps"
cc -O2 -static -s tools/aether-core.c -o "$ROOT/bin/aether-core"

cat > "$ROOT/init" <<'INIT'
#!/bin/sh
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev 2>/dev/null || true
mount -t tmpfs tmpfs /run
clear
echo
echo "============================================================"
echo "                         AETHER"
echo "                       AI OPERATING SYSTEM"
echo "============================================================"
echo
echo "  Linux foundation    : ONLINE"
echo "  Aether Core         : STARTING"
echo "  AI system interface : PENDING"
echo
echo "  This is not a Linux distribution."
echo "  Linux provides the foundation. Aether provides the system."
echo "============================================================"
/bin/aether-core &
sleep 1
if [ -S /run/aether-core.sock ]; then
    echo "AETHER> Aether Core IPC: READY"
else
    echo "AETHER> Aether Core IPC: FAILED"
fi
export PATH=/bin:/sbin:/usr/bin:/usr/sbin
exec /bin/sh
INIT
chmod +x "$ROOT/init"
mkdir -p build
(cd "$ROOT" && find . -print0 | cpio --null -ov --format=newc | gzip -9) > build/initramfs.cpio.gz
