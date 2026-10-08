#!/usr/bin/env bash
set -euo pipefail

KERNEL_VERSION="${KERNEL_VERSION:-6.18.55}"
OUT="${OUT:-out}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="$OUT/gui-work"
KERNEL_SRC="$WORK/linux-$KERNEL_VERSION"
KERNEL_TARBALL="$WORK/linux-$KERNEL_VERSION.tar.xz"
ROOTFS="$WORK/rootfs"
INITRAMFS_ROOT="$WORK/initramfs"
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
./scripts/config --enable CONFIG_BLK_DEV_LOOP
./scripts/config --enable CONFIG_DEVTMPFS
./scripts/config --enable CONFIG_DEVTMPFS_MOUNT
./scripts/config --enable CONFIG_SERIAL_8250
./scripts/config --enable CONFIG_SERIAL_8250_CONSOLE
./scripts/config --enable CONFIG_DRM
./scripts/config --enable CONFIG_DRM_I915
./scripts/config --enable CONFIG_DRM_VIRTIO_GPU
./scripts/config --enable CONFIG_DRM_BOCHS
./scripts/config --enable CONFIG_FB
./scripts/config --enable CONFIG_FRAMEBUFFER_CONSOLE
./scripts/config --enable CONFIG_VT
./scripts/config --enable CONFIG_VT_CONSOLE
./scripts/config --enable CONFIG_INPUT_EVDEV
./scripts/config --enable CONFIG_SQUASHFS
./scripts/config --enable CONFIG_SQUASHFS_XZ
./scripts/config --enable CONFIG_ISO9660_FS
./scripts/config --enable CONFIG_TMPFS
make olddefconfig
make -j"$(nproc)" bzImage

rm -rf "$ROOTFS" "$INITRAMFS_ROOT" "$ISO_ROOT"
mkdir -p "$ROOTFS" "$INITRAMFS_ROOT"/{bin,dev,proc,sys,run,iso,lower,newroot,tmp} "$ISO_ROOT/boot/grub"

debootstrap --variant=minbase bookworm "$ROOTFS" http://deb.debian.org/debian

# debootstrap may leave runtime mountpoints absent; create them before packing the rootfs.
mkdir -p "$ROOTFS/dev" "$ROOTFS/proc" "$ROOTFS/sys" "$ROOTFS/run" "$ROOTFS/tmp"

mount --bind /dev "$ROOTFS/dev"
mount -t proc proc "$ROOTFS/proc"
mount -t sysfs sysfs "$ROOTFS/sys"
mount --bind /run "$ROOTFS/run"
trap 'umount -lf "$ROOTFS/run" "$ROOTFS/sys" "$ROOTFS/proc" "$ROOTFS/dev" 2>/dev/null || true' EXIT
cp /etc/resolv.conf "$ROOTFS/etc/resolv.conf"

cat > "$ROOTFS/etc/apt/apt.conf.d/99aether-gui" <<'APT'
APT::Install-Recommends "false";
APT::Install-Suggests "false";
APT::Get::Assume-Yes "true";
APT

chroot "$ROOTFS" /usr/bin/env DEBIAN_FRONTEND=noninteractive apt-get update
chroot "$ROOTFS" /usr/bin/env DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends -y   xserver-xorg-core xinit openbox xterm xfonts-base dbus

mkdir -p "$ROOTFS/etc/X11"
cat > "$ROOTFS/etc/X11/Xwrapper.config" <<'XWRAP'
allowed_users=anybody
needs_root_rights=yes
XWRAP

cat > "$ROOTFS/etc/hostname" <<'HOST'
aether-gui-test
HOST

cat > "$ROOTFS/usr/local/bin/aether-gui-session" <<'SESSION'
#!/bin/sh
set -eu
mkdir -p /run/aether /tmp/aether
openbox --sm-disable &
WM_PID=$!
sleep 2
printf '%s
' 'AETHER_GUI=READY' > /run/aether/state
printf '%s
' 'AETHER_GUI_DISPLAY=:0' >> /run/aether/state
printf '%s
' 'AETHER_GUI_WM=OPENBOX' >> /run/aether/state
if [ -e /dev/ttyS0 ]; then
  printf '%s
' 'AETHER_GUI=READY' > /dev/ttyS0
fi
xterm -geometry 100x30+20+20 -title 'Aether GUI Test' -e /bin/sh -c 'printf "\nAETHER GUI TEST\n"; printf "Xorg + Openbox + Xterm are running.\n\n"; exec /bin/sh'
wait "$WM_PID"
SESSION
chmod 0755 "$ROOTFS/usr/local/bin/aether-gui-session"

cat > "$ROOTFS/sbin/init" <<'INIT'
#!/bin/sh
set -eu
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev
mount -t tmpfs tmpfs /run
mount -t tmpfs tmpfs /tmp
mkdir -p /run/aether /tmp/aether
exec /usr/bin/xinit /usr/local/bin/aether-gui-session -- :0 vt1 -nolisten tcp
INIT
chmod 0755 "$ROOTFS/sbin/init"

umount -lf "$ROOTFS/run" "$ROOTFS/sys" "$ROOTFS/proc" "$ROOTFS/dev" 2>/dev/null || true
trap - EXIT

cp "$(command -v busybox)" "$INITRAMFS_ROOT/bin/busybox"
chmod 0755 "$INITRAMFS_ROOT/bin/busybox"
cat > "$INITRAMFS_ROOT/init" <<'INITRAMFS'
#!/bin/busybox sh
set -eu
/bin/busybox mount -t devtmpfs devtmpfs /dev
/bin/busybox mount -t proc proc /proc
/bin/busybox mount -t sysfs sysfs /sys
/bin/busybox mkdir -p /iso /lower /newroot
for dev in /dev/sr0 /dev/vda /dev/vdb; do
  [ -b "$dev" ] || continue
  /bin/busybox mount -o ro "$dev" /iso 2>/dev/null && break || true
done
if [ ! -f /iso/rootfs.squashfs ]; then
  echo "AETHER_GUI_ROOTFS_NOT_FOUND"
  exec /bin/busybox sh
fi
/bin/busybox mount -t squashfs -o loop /iso/rootfs.squashfs /lower
/bin/busybox mount --move /dev /newroot/dev
/bin/busybox mount --move /proc /newroot/proc
/bin/busybox mount --move /sys /newroot/sys
exec /bin/busybox switch_root /newroot /sbin/init
INITRAMFS
chmod 0755 "$INITRAMFS_ROOT/init"

(
  cd "$INITRAMFS_ROOT"
  find . -print0 | cpio --null -o -H newc 2>/dev/null | gzip -9
) > "$WORK/initramfs.cpio.gz"

mksquashfs "$ROOTFS" "$WORK/rootfs.squashfs" -comp xz -noappend >/dev/null
cp arch/x86/boot/bzImage "$ISO_ROOT/boot/vmlinuz"
cp "$WORK/initramfs.cpio.gz" "$ISO_ROOT/boot/initramfs.cpio.gz"
cp "$WORK/rootfs.squashfs" "$ISO_ROOT/rootfs.squashfs"

cat > "$ISO_ROOT/boot/grub/grub.cfg" <<'GRUB'
set timeout=0
set default=0
menuentry "Aether GUI Test" {
    linux /boot/vmlinuz console=ttyS0,115200n8
    initrd /boot/initramfs.cpio.gz
}
GRUB

grub-mkrescue -o "$ROOT/$ARTIFACT" "$ISO_ROOT" >/dev/null
echo "GUI_TEST_IMAGE=$ROOT/$ARTIFACT"
