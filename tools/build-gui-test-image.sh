#!/usr/bin/env bash
set -euo pipefail

KERNEL_VERSION="${KERNEL_VERSION:-6.18.55}"
OUT="${OUT:-out}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="$OUT/desktop-work"
KERNEL_SRC="$WORK/linux-$KERNEL_VERSION"
KERNEL_TARBALL="$WORK/linux-$KERNEL_VERSION.tar.xz"
ROOTFS="$WORK/rootfs"
ISO_ROOT="$WORK/iso"
ARTIFACT="$OUT/aether-gui-test-$KERNEL_VERSION-x86_64.iso"

mkdir -p "$WORK" "$OUT"
if [ ! -d "$KERNEL_SRC" ]; then
  curl -fL --retry 3 -o "$KERNEL_TARBALL" "https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-$KERNEL_VERSION.tar.xz"
  tar -xJf "$KERNEL_TARBALL" -C "$WORK"
fi

cd "$KERNEL_SRC"
make mrproper
make x86_64_defconfig
./scripts/config --enable CONFIG_BLK_DEV_INITRD
./scripts/config --enable CONFIG_DEVTMPFS
./scripts/config --enable CONFIG_DEVTMPFS_MOUNT
./scripts/config --enable CONFIG_SERIAL_8250
./scripts/config --enable CONFIG_SERIAL_8250_CONSOLE
./scripts/config --enable CONFIG_DRM
./scripts/config --enable CONFIG_DRM_I915
./scripts/config --enable CONFIG_DRM_VIRTIO_GPU
./scripts/config --enable CONFIG_FB
./scripts/config --enable CONFIG_FRAMEBUFFER_CONSOLE
./scripts/config --enable CONFIG_VT
./scripts/config --enable CONFIG_VT_CONSOLE
./scripts/config --enable CONFIG_INPUT_EVDEV
./scripts/config --enable CONFIG_INPUT_KEYBOARD
./scripts/config --enable CONFIG_INPUT_MOUSE
./scripts/config --enable CONFIG_USB_HID
./scripts/config --enable CONFIG_USB_XHCI_HCD
./scripts/config --enable CONFIG_TMPFS
make olddefconfig
make -j"$(nproc)" bzImage

rm -rf "$ROOTFS" "$ISO_ROOT"
mkdir -p "$ROOTFS"/{dev,proc,sys,run,tmp} "$ISO_ROOT/boot/grub"

debootstrap --variant=minbase bookworm "$ROOTFS" http://deb.debian.org/debian

mount --bind /dev "$ROOTFS/dev"
mount -t proc proc "$ROOTFS/proc"
mount -t sysfs sysfs "$ROOTFS/sys"
mount --bind /run "$ROOTFS/run"
trap 'umount -lf "$ROOTFS/run" "$ROOTFS/sys" "$ROOTFS/proc" "$ROOTFS/dev" 2>/dev/null || true' EXIT

cp /etc/resolv.conf "$ROOTFS/etc/resolv.conf"
cat > "$ROOTFS/etc/apt/apt.conf.d/99aether-desktop" <<'APT'
APT::Install-Recommends "false";
APT::Install-Suggests "false";
APT::Get::Assume-Yes "true";
APT

chroot "$ROOTFS" /usr/bin/env DEBIAN_FRONTEND=noninteractive apt-get update
chroot "$ROOTFS" /usr/bin/env DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends -y \
  xserver-xorg-core xinit openbox xterm xfonts-base

mkdir -p "$ROOTFS/etc/X11/xorg.conf.d"
cat > "$ROOTFS/etc/X11/xorg.conf.d/20-aether-modesetting.conf" <<'XORG'
Section "Device"
    Identifier "Aether GPU"
    Driver "modesetting"
EndSection
XORG

cat > "$ROOTFS/etc/hostname" <<'HOST'
aether-desktop
HOST

cat > "$ROOTFS/usr/local/bin/aether-desktop-session" <<'SESSION'
#!/bin/sh
set -eu

mkdir -p /run/aether /tmp/aether

openbox --sm-disable &
WM_PID=$!

sleep 2
kill -0 "$WM_PID"

xterm -geometry 100x30+20+20 -title 'Aether Desktop' \
  -e /bin/sh -c 'printf "\nAETHER DESKTOP\n"; printf "Xorg + Openbox + Xterm are running.\n\n"; exec /bin/sh' &
XTERM_PID=$!

sleep 3
kill -0 "$XTERM_PID"

printf '%s\n' 'AETHER_DESKTOP=READY' > /run/aether/state
printf '%s\n' 'AETHER_DESKTOP_DISPLAY=:0' >> /run/aether/state
printf '%s\n' 'AETHER_DESKTOP_WM=OPENBOX' >> /run/aether/state

if [ -e /dev/ttyS0 ]; then
    printf '%s\n' 'AETHER_DESKTOP=READY' > /dev/ttyS0\n    printf '%s\n' 'AETHER_GUI=READY' > /dev/ttyS0
fi

wait "$XTERM_PID"
SESSION
chmod 0755 "$ROOTFS/usr/local/bin/aether-desktop-session"

cat > "$ROOTFS/sbin/init" <<'INIT'
#!/bin/sh
set -eu

mount -t devtmpfs devtmpfs /dev
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t tmpfs tmpfs /run
mount -t tmpfs tmpfs /tmp

mkdir -p /run/aether /tmp/aether

exec /usr/bin/xinit /usr/local/bin/aether-desktop-session -- :0 vt1 -nolisten tcp
INIT
chmod 0755 "$ROOTFS/sbin/init"

umount -lf "$ROOTFS/run" "$ROOTFS/sys" "$ROOTFS/proc" "$ROOTFS/dev" 2>/dev/null || true
trap - EXIT

rm -rf "$ROOTFS/var/lib/apt/lists/"*
rm -rf "$ROOTFS/var/cache/apt/"archives/*

(
  cd "$ROOTFS"
  find . -print0 | cpio --null -o -H newc 2>/dev/null | gzip -9
) > "$WORK/desktop-initramfs.cpio.gz"

cp arch/x86/boot/bzImage "$ISO_ROOT/boot/vmlinuz"
cp "$WORK/desktop-initramfs.cpio.gz" "$ISO_ROOT/boot/initramfs.cpio.gz"

cat > "$ISO_ROOT/boot/grub/grub.cfg" <<'GRUB'
set timeout=0
set default=0

menuentry "Aether Desktop" {
    linux /boot/vmlinuz console=ttyS0,115200n8
    initrd /boot/initramfs.cpio.gz
}
GRUB

grub-mkrescue -o "$ROOT/$ARTIFACT" "$ISO_ROOT" >/dev/null
echo "DESKTOP_TEST_IMAGE=$ROOT/$ARTIFACT"
