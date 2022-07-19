.PHONY: love default all clean test install run kernel

MNT			= ./mnt/

LLVMPATH	= `brew --prefix`/opt/llvm/bin
CLANGFLAGS	= -Wall -O2 -ffreestanding -nostdinc -nostdlib -mcpu=cortex-a72+nosimd
OBJCOPY		= gobjcopy

default: kernel

clean:

test:

$(MNT):
	mkdir $(MNT)

kernel:
	(cd kernel; cargo build --release --target aarch64-unknown-none)

install: $(MNT) kernel
	$(OBJCOPY) -O binary kernel/target/aarch64-unknown-none/release/rydia mnt/kernel8.img

run:
	qemu-system-aarch64 -M raspi3b \
-kernel mnt/kernel8.img -dtb assets/dtb/bcm2710-rpi-3-b.dtb \
-usb -device usb-kbd -device usb-tablet \
-serial mon:stdio

# -drive if=none,id=stick,format=raw,file=fat:rw:$(MNT) -device usb-storage,drive=stick \

