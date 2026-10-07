KERNEL_VERSION ?= 6.18.55
OUT ?= out

.PHONY: boot-image clean

boot-image:
	KERNEL_VERSION=$(KERNEL_VERSION) OUT=$(OUT) ./tools/build-boot-image.sh

clean:
	rm -rf $(OUT)
