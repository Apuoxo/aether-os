BUILD_DIR  = build
KERNEL_BIN = $(BUILD_DIR)/aether.bin
KERNEL_LIB = $(BUILD_DIR)/libaether.a
ISR_OBJ    = $(BUILD_DIR)/isr.o
ISO        = $(BUILD_DIR)/aether.iso
ISO_DIR    = $(BUILD_DIR)/isofiles

.PHONY: all clean run iso kernel

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/boot.o: src/arch/x86_64/boot.s | $(BUILD_DIR)
	nasm -f elf64 $< -o $@

RUST_SRCS := $(shell find src -type f -name '*.rs')

$(BUILD_DIR)/calculator_blob.rs: $(BUILD_DIR)/calculator.elf | $(BUILD_DIR)
	python3 -c "import pathlib; pathlib.Path('$(BUILD_DIR)/calculator_blob.rs').write_text('pub static CALCULATOR_ELF: &[u8] = include_bytes!(\\\"calculator.elf\\\");\\n')"

$(BUILD_DIR)/iwlwifi-2030-6.ucode: | $(BUILD_DIR)
	@if [ ! -s $@ ]; then curl -L --fail --silent --show-error "https://kernel.googlesource.com/pub/scm/linux/kernel/git/firmware/linux-firmware.git/+/refs/tags/20250211/iwlwifi-2030-6.ucode?format=TEXT" | base64 -d > $@; fi
	test "$(wc -c < "$@")" -eq 707392

$(BUILD_DIR)/calculator.elf: | $(BUILD_DIR)
	@if [ ! -f $@ ]; then printf '%s\\n' 'calculator ELF not supplied; building kernel without bundled Calculator' >&2; printf '' > $@; fi

$(KERNEL_LIB): $(RUST_SRCS) $(BUILD_DIR)/calculator_blob.rs $(BUILD_DIR)/iwlwifi-2030-6.ucode | $(BUILD_DIR)
	rustc --edition 2021 \
		-C opt-level=s -C panic=abort \
		-C overflow-checks=off -C debug-assertions=off \
		-C force-frame-pointers=no \
		--target x86_64-unknown-linux-gnu \
		-C relocation-model=static \
		--crate-type=staticlib -o $@ src/main.rs

$(ISR_OBJ): src/arch/x86_64/isr.s | $(BUILD_DIR)
	nasm -f elf64 $< -o $@

$(KERNEL_BIN): $(BUILD_DIR)/boot.o $(KERNEL_LIB) $(ISR_OBJ) linker/linker.ld
	ld -n -T linker/linker.ld -o $@ $(BUILD_DIR)/boot.o $(KERNEL_LIB) $(ISR_OBJ)

kernel: $(KERNEL_BIN)
	@ls -lh $(KERNEL_BIN)

iso: $(KERNEL_BIN)
	rm -rf $(ISO_DIR)
	mkdir -p $(ISO_DIR)/boot/grub
	cp $(KERNEL_BIN) $(ISO_DIR)/boot/aether.bin
	printf 'set timeout=0\nset default=0\nterminal_output console\nmenuentry "Aether" {\n  multiboot2 /boot/aether.bin\n  boot\n}\n' > $(ISO_DIR)/boot/grub/grub.cfg
	# First-boot diagnostic image: BIOS/CSM only. Do not invoke x86_64-efi GRUB.
	grub-mkrescue -d /usr/lib/grub/i386-pc --install-modules="iso9660 multiboot2 normal terminal" --themes= --fonts= --locales= -o $(ISO) $(ISO_DIR)
	@ls -lh $(ISO)

run: iso
	timeout 10 qemu-system-x86_64 -cdrom $(ISO) -serial stdio -display none -m 128M -no-reboot 2>/dev/null || true

clean:
	rm -rf $(BUILD_DIR)
