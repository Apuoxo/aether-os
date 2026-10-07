LINUX_DIR := linux

.PHONY: help linux-config linux-build clean

help:
	@echo "Aether OS Linux build"
	@echo "  make linux-config  - configure x86_64 Linux"
	@echo "  make linux-build   - build Linux kernel"
	@echo "  make clean         - clean Linux build artifacts"

linux-config:
	$(MAKE) -C $(LINUX_DIR) x86_64_defconfig

linux-build:
	$(MAKE) -C $(LINUX_DIR) -j$$(nproc)

clean:
	$(MAKE) -C $(LINUX_DIR) clean
