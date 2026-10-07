#!/usr/bin/env bash
set -euo pipefail

KERNEL_VERSION="${KERNEL_VERSION:-6.18.55}"
OUT="${OUT:-out}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="$OUT/work"
KERNEL_SRC="$WORK/linux-$KERNEL_VERSION"
KERNEL_TARBALL="$WORK/linux-$KERNEL_VERSION.tar.xz"
ROOTFS="$WORK/rootfs"
INITRAMFS_ROOT="$WORK/initramfs"
ISO_ROOT="$WORK/iso"
ARTIFACT="$OUT/aether-os-$KERNEL_VERSION-x86_64.iso"

mkdir -p "$WORK" "$OUT"

if [ ! -d "$KERNEL_SRC" ]; then
  curl -fL --retry 3 -o "$KERNEL_TARBALL"     "https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-$KERNEL_VERSION.tar.xz"
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
./scripts/config --enable CONFIG_VIRTIO
./scripts/config --enable CONFIG_VIRTIO_PCI
./scripts/config --enable CONFIG_VIRTIO_BLK
./scripts/config --enable CONFIG_VIRTIO_NET
./scripts/config --enable CONFIG_VIRTIO_CONSOLE
./scripts/config --enable CONFIG_DRM
./scripts/config --enable CONFIG_DRM_VIRTIO_GPU
./scripts/config --enable CONFIG_FB
./scripts/config --enable CONFIG_FRAMEBUFFER_CONSOLE
./scripts/config --enable CONFIG_VT
./scripts/config --enable CONFIG_VT_CONSOLE
./scripts/config --enable CONFIG_INPUT_EVDEV
./scripts/config --enable CONFIG_USB_HID
./scripts/config --enable CONFIG_SQUASHFS
./scripts/config --enable CONFIG_ISO9660_FS
./scripts/config --enable CONFIG_OVERLAY_FS
./scripts/config --enable CONFIG_EXT4_FS
./scripts/config --enable CONFIG_UNIX
./scripts/config --enable CONFIG_TMPFS
./scripts/config --enable CONFIG_9P_FS
./scripts/config --enable CONFIG_NET_9P
./scripts/config --enable CONFIG_NET_9P_VIRTIO
make olddefconfig
make -j"$(nproc)" bzImage

rm -rf "$ROOTFS" "$INITRAMFS_ROOT" "$ISO_ROOT"
mkdir -p "$ROOTFS" "$INITRAMFS_ROOT"/{bin,dev,proc,sys,run,tmp,iso,lower,rw,newroot}

debootstrap --variant=minbase bookworm "$ROOTFS" http://deb.debian.org/debian

cat > "$ROOTFS/etc/apt/sources.list" <<'SOURCES'
deb http://deb.debian.org/debian bookworm main contrib non-free-firmware
deb http://deb.debian.org/debian bookworm-updates main contrib non-free-firmware
deb http://security.debian.org/debian-security bookworm-security main contrib non-free-firmware
SOURCES

cat > "$ROOTFS/etc/hostname" <<'HOSTNAME'
aether
HOSTNAME

cat > "$ROOTFS/etc/hosts" <<'HOSTS'
127.0.0.1 localhost
127.0.1.1 aether
::1 localhost ip6-localhost ip6-loopback
HOSTS

cat > "$ROOTFS/etc/apt/apt.conf.d/99aether-noninteractive" <<'APT'
APT::Install-Recommends "true";
APT::Install-Suggests "false";
APT::Get::Assume-Yes "true";
APT

mount --bind /dev "$ROOTFS/dev"
mount -t proc proc "$ROOTFS/proc"
mount -t sysfs sysfs "$ROOTFS/sys"
mount --bind /run "$ROOTFS/run"
trap 'umount -lf "$ROOTFS/run" "$ROOTFS/sys" "$ROOTFS/proc" "$ROOTFS/dev" 2>/dev/null || true' EXIT

cp /etc/resolv.conf "$ROOTFS/etc/resolv.conf"

chroot "$ROOTFS" /usr/bin/env DEBIAN_FRONTEND=noninteractive apt-get update
chroot "$ROOTFS" /usr/bin/env DEBIAN_FRONTEND=noninteractive apt-get install -y   systemd systemd-sysv dbus dbus-x11   xserver-xorg xinit xfce4 xfce4-terminal lightdm   network-manager sudo bash-completion   pciutils usbutils iproute2 iputils-ping procps psmisc   curl ca-certificates nano less   firmware-linux-free

chroot "$ROOTFS" useradd -m -s /bin/bash aether
chroot "$ROOTFS" usermod -aG audio,video,netdev,plugdev,sudo aether
chroot "$ROOTFS" passwd -d aether

mkdir -p "$ROOTFS/etc/lightdm/lightdm.conf.d"
cat > "$ROOTFS/etc/lightdm/lightdm.conf.d/50-aether.conf" <<'LIGHTDM'
[Seat:*]
autologin-user=aether
autologin-user-timeout=0
user-session=xfce
LIGHTDM

mkdir -p "$ROOTFS/usr/local/libexec"
cat > "$ROOTFS/usr/local/libexec/aether-ai-bridge" <<'AI'
#!/bin/sh
set -eu
mkdir -p /run/aether
printf '%s
' 'AETHER_AI_BRIDGE=READY' > /run/aether/state
printf '%s
' 'AETHER_AI_INTERFACE=/run/aether' >> /run/aether/state
printf '%s
' 'AETHER_AI_CONTROL=CAPABILITY_BOUND' >> /run/aether/state
exec /usr/bin/tail -f /dev/null
AI
chmod 0755 "$ROOTFS/usr/local/libexec/aether-ai-bridge"

cat > "$ROOTFS/etc/systemd/system/aether-ai.service" <<'SERVICE'
[Unit]
Description=Aether AI system integration boundary
After=basic.target dbus.service
Wants=dbus.service

[Service]
Type=simple
ExecStart=/usr/local/libexec/aether-ai-bridge
Restart=on-failure
RestartSec=2

[Install]
WantedBy=multi-user.target
SERVICE

chroot "$ROOTFS" systemctl enable aether-ai.service
chroot "$ROOTFS" systemctl enable lightdm
chroot "$ROOTFS" systemctl set-default graphical.target

umount -lf "$ROOTFS/run" "$ROOTFS/sys" "$ROOTFS/proc" "$ROOTFS/dev" 2>/dev/null || true
trap - EXIT

cp "$(command -v busybox)" "$INITRAMFS_ROOT/bin/busybox"
chmod 0755 "$INITRAMFS_ROOT/bin/busybox"

cat > "$INITRAMFS_ROOT/init" <<'INIT'
#!/bin/busybox sh
set -eu

/bin/busybox mount -t proc proc /proc
/bin/busybox mount -t sysfs sysfs /sys
/bin/busybox mount -t devtmpfs devtmpfs /dev
/bin/busybox mkdir -p /run /iso /lower /rw /newroot

for dev in /dev/sr0 /dev/vda /dev/vdb; do
    [ -b "$dev" ] || continue
    /bin/busybox mount -o ro "$dev" /iso 2>/dev/null && break || true
done

if [ ! -f /iso/rootfs.squashfs ]; then
    echo "AETHER_ROOTFS_NOT_FOUND"
    exec /bin/busybox sh
fi

/bin/busybox mount -t squashfs -o loop /iso/rootfs.squashfs /lower
/bin/busybox mount -t tmpfs tmpfs /rw
/bin/busybox mkdir -p /rw/upper /rw/work
/bin/busybox mount -t overlay overlay -o lowerdir=/lower,upperdir=/rw/upper,workdir=/rw/work /newroot

/bin/busybox mount --move /dev /newroot/dev
/bin/busybox mount --move /proc /newroot/proc
/bin/busybox mount --move /sys /newroot/sys
/bin/busybox mount --move /run /newroot/run
exec /bin/busybox switch_root /newroot /sbin/init
INIT
chmod 0755 "$INITRAMFS_ROOT/init"

(
  cd "$INITRAMFS_ROOT"
  find . -print0 | cpio --null -o -H newc 2>/dev/null | gzip -9
) > "$WORK/initramfs.cpio.gz"

mksquashfs "$ROOTFS" "$WORK/rootfs.squashfs" -comp xz -noappend >/dev/null

mkdir -p "$ISO_ROOT/boot/grub"
cp arch/x86/boot/bzImage "$ISO_ROOT/boot/vmlinuz"
cp "$WORK/initramfs.cpio.gz" "$ISO_ROOT/boot/initramfs.cpio.gz"
cp "$WORK/rootfs.squashfs" "$ISO_ROOT/rootfs.squashfs"

cat > "$ISO_ROOT/boot/grub/grub.cfg" <<'GRUB'
set timeout=0
set default=0

menuentry "Aether OS Linux Desktop" {
    linux /boot/vmlinuz console=ttyS0,115200n8
    initrd /boot/initramfs.cpio.gz
}
GRUB

grub-mkrescue -o "$ROOT/$ARTIFACT" "$ISO_ROOT" >/dev/null

echo "BOOT_IMAGE=$ROOT/$ARTIFACT"
