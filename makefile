all: bootloader kernel image

bootloader:
	nasm -f bin boot/src/boot.asm -o bin/boot.bin

kernel:
	nasm -f elf64 boot/src/kernel_entry.asm -o bin/kernel_entry.o
	cargo build --target x86_64-unknown-none --release
	ld -T linker.ld -Map bin/kernel.map -o bin/kernel.bin bin/kernel_entry.o target/x86_64-unknown-none/release/libk_os.a

image:
	cat bin/boot.bin bin/kernel.bin boot/src/zeroes.asm > bin/os.bin

run:
	qemu-system-x86_64 -no-reboot -drive format=raw,file=bin/os.bin

clean:
	rm bin/*
