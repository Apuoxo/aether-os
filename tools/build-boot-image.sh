#!/usr/bin/env bash
set -euo pipefail

KERNEL_VERSION="${KERNEL_VERSION:-6.18.55}"
OUT="${OUT:-out}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="$OUT/work"
KERNEL_SRC="$WORK/linux-$KERNEL_VERSION"
KERNEL_TARBALL="$WORK/linux-$KERNEL_VERSION.tar.xz"
ISO_ROOT="$WORK/iso"
INITRAMFS_ROOT="$WORK/initramfs"
ARTIFACT="$OUT/aether-os-$KERNEL_VERSION-x86_64.iso"

mkdir -p "$WORK" "$OUT"

if [ ! -d "$KERNEL_SRC" ]; then
  curl -fL --retry 3 -o "$KERNEL_TARBALL" \
    "https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-$KERNEL_VERSION.tar.xz"
  tar -xJf "$KERNEL_TARBALL" -C "$WORK"
fi

cd "$KERNEL_SRC"
make mrproper
make x86_64_defconfig

# Broad Linux baseline: these are boot requirements, not a reduced kernel policy.
./scripts/config --enable CONFIG_BLK_DEV_INITRD
./scripts/config --enable CONFIG_DEVTMPFS
./scripts/config --enable CONFIG_DEVTMPFS_MOUNT
./scripts/config --enable CONFIG_SERIAL_8250
./scripts/config --enable CONFIG_SERIAL_8250_CONSOLE
./scripts/config --enable CONFIG_VIRTIO
./scripts/config --enable CONFIG_VIRTIO_PCI
./scripts/config --enable CONFIG_VIRTIO_BLK
./scripts/config --enable CONFIG_VIRTIO_NET
./scripts/config --enable CONFIG_VIRTIO_CONSOLE
./scripts/config --enable CONFIG_9P_FS
./scripts/config --enable CONFIG_NET_9P
./scripts/config --enable CONFIG_NET_9P_VIRTIO
make olddefconfig
make -j"$(nproc)" bzImage

rm -rf "$INITRAMFS_ROOT" "$ISO_ROOT"
mkdir -p "$INITRAMFS_ROOT"/{bin,dev,etc,proc,sys,run,tmp}
cp "$(command -v busybox)" "$INITRAMFS_ROOT/bin/busybox"
chmod 0755 "$INITRAMFS_ROOT/bin/busybox"

cat > "$INITRAMFS_ROOT/init" <<'INIT'
#!/bin/busybox sh
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev 2>/dev/null || true
echo
echo "AETHER_BOOT_OK"
echo "Aether Linux foundation is running."
echo "Kernel: $(uname -r)"
echo "CPU: $(nproc) logical CPUs"
echo "AI integration: not started in bootstrap image"
echo
exec /bin/busybox sh
INIT
chmod 0755 "$INITRAMFS_ROOT/init"

(
  cd "$INITRAMFS_ROOT"
  find . -print0 | cpio --null -o -H newc 2>/dev/null | gzip -9
) > "$WORK/initramfs.cpio.gz"

mkdir -p "$ISO_ROOT/boot/grub"
cp arch/x86/boot/bzImage "$ISO_ROOT/boot/vmlinuz"
cp "$WORK/initramfs.cpio.gz" "$ISO_ROOT/boot/initramfs.cpio.gz"

cat > "$ISO_ROOT/boot/grub/grub.cfg" <<'GRUB'
set timeout=0
set default=0

menuentry "Aether OS Linux Foundation" {
    linux /boot/vmlinuz console=ttyS0,115200n8 rdinit=/init
    initrd /boot/initramfs.cpio.gz
}
GRUB

grub-mkrescue -o "$ROOT/$ARTIFACT" "$ISO_ROOT" >/dev/null

echo "BOOT_IMAGE=$ROOT/$ARTIFACT"
