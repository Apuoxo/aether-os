#!/usr/bin/env bash
set -euo pipefail
rm -rf build/iso
mkdir -p build/iso/boot/grub
cp linux/arch/x86/boot/bzImage build/iso/boot/vmlinuz
cp build/initramfs.cpio.gz build/iso/boot/initramfs.cpio.gz
cat > build/iso/boot/grub/grub.cfg <<'GRUB'
set timeout=0
set default=0
menuentry "Aether AI OS" {
    linux /boot/vmlinuz console=tty0 console=ttyS0,115200n8 loglevel=4
    initrd /boot/initramfs.cpio.gz
}
GRUB
grub-mkrescue -o build/aether-os.iso build/iso >/dev/null
