BUILD_DIR  = build
MEDIA_DIR = $(BUILD_DIR)/media
MEDIA_WAV = $(MEDIA_DIR)/TEST.WAV
MEDIA_MP3 = $(MEDIA_DIR)/TEST.MP3
MEDIA_BEETHOVEN_SRC = $(MEDIA_DIR)/BEETHOVEN_SOURCE.OGG
MEDIA_BUNDLE = $(BUILD_DIR)/media_builtin.rs
WALLPAPER = $(BUILD_DIR)/wallpaper.rgb565
CURSOR_BUNDLE = $(BUILD_DIR)/cursor_builtin.rs

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
	test "$$(wc -c < "$(BUILD_DIR)/wallpaper.rgb565")" -eq 2098176
	rm -f "$(BUILD_DIR)/wallpaper-source.jpg"

$(CURSOR_BUNDLE): | $(BUILD_DIR)
	mkdir -p "$(BUILD_DIR)/cursor_src" "$(BUILD_DIR)/cursor_png"
	set -e; for spec in "default:red:#E81123" "default:blue:#0078D7" "default:green:#107C10" "default:gold:#FFB900" "default:purple:#5C2D91" "default:cyan:#00B7C3" "pointer:red:#E81123" "pointer:blue:#0078D7" "pointer:green:#107C10" "pointer:gold:#FFB900" "pointer:purple:#5C2D91" "pointer:cyan:#00B7C3" "crosshair:red:#E81123" "crosshair:blue:#0078D7" "crosshair:green:#107C10" "crosshair:gold:#FFB900" "crosshair:purple:#5C2D91" "crosshair:cyan:#00B7C3" "text:red:#E81123" "text:blue:#0078D7" "text:green:#107C10" "text:gold:#FFB900" "text:purple:#5C2D91" "text:cyan:#00B7C3"; do \
		shape=${spec%%:*}; rest=${spec#*:}; name=${rest%%:*}; color=${rest#*:}; \
		curl -L --fail --retry 5 --retry-delay 2 -A "AetherOS/1.0" "https://raw.githubusercontent.com/phisch/phinger-cursors/master/theme/light/${shape}_32.svg" -o "$(BUILD_DIR)/cursor_src/${shape}.svg"; \
		sed "s/#FBFBFB/${color}/g" "$(BUILD_DIR)/cursor_src/${shape}.svg" > "$(BUILD_DIR)/cursor_src/${shape}_${name}.svg"; \
		rsvg-convert -w 16 -h 16 "$(BUILD_DIR)/cursor_src/${shape}_${name}.svg" -o "$(BUILD_DIR)/cursor_png/${shape}_${name}.png"; \
	done
	python3 -c "from pathlib import Path; specs='default:red default:blue default:green default:gold default:purple default:cyan pointer:red pointer:blue pointer:green pointer:gold pointer:purple pointer:cyan crosshair:red crosshair:blue crosshair:green crosshair:gold crosshair:purple crosshair:cyan text:red text:blue text:green text:gold text:purple text:cyan'.split(); p=Path('$(BUILD_DIR)/cursor_builtin.rs'); p.write_text('pub const COUNT: usize = 24;\\n'+''.join([f'pub static C{i}: &[u8] = include_bytes!(\\"cursor_png/{s.replace(\\":\\", \\"_\\")}.png\\");\\n' for i,s in enumerate(specs)]))"

$(KERNEL_LIB): $(RUST_SRCS) $(BUILD_DIR)/calculator_blob.rs $(BUILD_DIR)/iwlwifi-2030-6.ucode $(MEDIA_BUNDLE) $(WALLPAPER) $(CURSOR_BUNDLE) | $(BUILD_DIR)
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
