BUILD_DIR  = build
MEDIA_DIR = $(BUILD_DIR)/media
MEDIA_WAV = $(MEDIA_DIR)/TEST.WAV
MEDIA_MP3 = $(MEDIA_DIR)/TEST.MP3
MEDIA_BEETHOVEN_SRC = $(MEDIA_DIR)/BEETHOVEN_SOURCE.OGG
MEDIA_BUNDLE = $(BUILD_DIR)/media_builtin.rs
WALLPAPER = $(BUILD_DIR)/wallpaper.rgb565

$(MEDIA_DIR): | $(BUILD_DIR)
	mkdir -p $(MEDIA_DIR)

$(MEDIA_WAV): | $(MEDIA_DIR)
	curl -L --fail --retry 5 --retry-delay 2 -A "AetherOS/1.0" "https://upload.wikimedia.org/wikipedia/commons/c/c2/The_English_word_EXAMPLE.wav" -o $@
	test -s "$@"

$(MEDIA_BEETHOVEN_SRC): | $(MEDIA_DIR)
	curl -L --fail --retry 5 --retry-delay 2 -A "AetherOS/1.0" "https://upload.wikimedia.org/wikipedia/commons/f/f5/Moonlight_Sonata_Allegretto.ogg" -o "$@"
	test -s "$@"

$(MEDIA_MP3): $(MEDIA_BEETHOVEN_SRC) | $(MEDIA_DIR)
	ffmpeg -hide_banner -loglevel error -i "$(MEDIA_BEETHOVEN_SRC)" -ac 2 -ar 44100 -c:a libmp3lame -b:a 192k -y "$@"
	test -s "$@"

$(MEDIA_BUNDLE): $(MEDIA_WAV) $(MEDIA_MP3) | $(BUILD_DIR)
	python3 -c "import pathlib; pathlib.Path('$@').write_text('pub static TEST_WAV: &[u8] = include_bytes!(\\\"media/TEST.WAV\\\");\\npub static TEST_MP3: &[u8] = include_bytes!(\\\"media/TEST.MP3\\\");\\n')"

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
	@if [ ! -s $@ ]; then \
		curl -L --fail --silent --show-error --retry 8 --retry-delay 2 --retry-all-errors \
			"https://raw.githubusercontent.com/pop-os/linux-firmware/master/intel/iwlwifi/iwlwifi-2030-6.ucode" \
			-o "$@.tmp"; \
		mv "$@.tmp" "$@"; \
	fi
	test "$$(wc -c < "$@")" -eq 707392
$(BUILD_DIR)/calculator.elf: | $(BUILD_DIR)
	@if [ ! -f $@ ]; then printf '%s\\n' 'calculator ELF not supplied; building kernel without bundled Calculator' >&2; printf '' > $@; fi

$(WALLPAPER): | $(BUILD_DIR)
	curl -L --fail --retry 5 --retry-delay 2 -A "AetherOS/1.0" "https://upload.wikimedia.org/wikipedia/commons/7/7b/Comeragh_Mountains_Lake.jpg" -o "$(BUILD_DIR)/wallpaper-source.jpg"
	ffmpeg -hide_banner -loglevel error -i "$(BUILD_DIR)/wallpaper-source.jpg" -vf "scale=1366:768" -pix_fmt rgb565le -f rawvideo -y "$@"
	test "$(wc -c < "$@")" -eq 2098176
	rm -f "$(BUILD_DIR)/wallpaper-source.jpg"

$(KERNEL_LIB): $(RUST_SRCS) $(BUILD_DIR)/calculator_blob.rs $(BUILD_DIR)/iwlwifi-2030-6.ucode $(MEDIA_BUNDLE) $(WALLPAPER) | $(BUILD_DIR)
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
