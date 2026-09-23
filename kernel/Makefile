BUILD_DIR  = build
KERNEL_BIN = $(BUILD_DIR)/aether.bin
ISO        = $(BUILD_DIR)/aether.iso
ISO_DIR    = $(BUILD_DIR)/isofiles

.PHONY: all clean run iso kernel

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/boot.o: src/arch/x86_64/boot.s | $(BUILD_DIR)
	nasm -f elf64 $< -o $@

RUST_SRCS := $(shell find src -type f -name '*.rs')

$(BUILD_DIR)/kernel.o: $(RUST_SRCS) | $(BUILD_DIR)
	rustc --edition 2021 \
		-C opt-level=s -C panic=abort \
		-C overflow-checks=off -C debug-assertions=off \
		-C force-frame-pointers=no \
		--target x86_64-unknown-linux-gnu \
		-C relocation-model=static \
		--emit=obj -o $@ src/main.rs

$(KERNEL_BIN): $(BUILD_DIR)/boot.o $(BUILD_DIR)/kernel.o linker/linker.ld
	ld -n -T linker/linker.ld -o $@ $(BUILD_DIR)/boot.o $(BUILD_DIR)/kernel.o

kernel: $(KERNEL_BIN)
	@ls -lh $(KERNEL_BIN)

iso: $(KERNEL_BIN)
	mkdir -p $(ISO_DIR)/boot/grub
	cp $(KERNEL_BIN) $(ISO_DIR)/boot/aether.bin
	printf 'set timeout=0\nset default=0\nmenuentry "Aether" {\n  multiboot2 /boot/aether.bin\n  boot\n}\n' > $(ISO_DIR)/boot/grub/grub.cfg
	grub-mkrescue -o $(ISO) $(ISO_DIR) 2>/dev/null
	@ls -lh $(ISO)

run: iso
	timeout 10 qemu-system-x86_64 -cdrom $(ISO) -serial stdio -display none -m 128M -no-reboot 2>/dev/null || true

clean:
	rm -rf $(BUILD_DIR)
