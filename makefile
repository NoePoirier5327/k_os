all: bootloader kernel linkage iso

bootloader:
	# Assemblage du bootloader.
	nasm -f elf64 boot/src/multiboot_header.asm -o bin/multiboot_header.o
	nasm -f elf64 boot/src/long_mode_init.asm -o bin/long_mode_init.o
	nasm -f elf64 boot/src/start.asm -o bin/start.o

kernel:
	# Compilation du noyau rust pour x86_64
	cargo build --target x86_64-unknown-none --release

linkage:
	# Linkage du bootloader et du kernel.
	ld -n -T linker.ld -Map bin/kernel.map -o bin/kernel.bin\
		bin/multiboot_header.o\
		bin/long_mode_init.o\
		bin/start.o\
		target/x86_64-unknown-none/release/libk_os.a

iso:
	# Création de la structure pour GRUB
	mkdir -p bin/isofiles/boot/grub
	cp bin/kernel.bin bin/isofiles/boot/
	cp boot/grub/grub.cfg bin/isofiles/boot/grub/
	
	# Création de l'image ISO
	grub-mkrescue -o bin/os.iso bin/isofiles

run:
	qemu-system-x86_64 -cdrom bin/os.iso -no-reboot -serial stdio

run-log:
	qemu-system-x86_64 -drive format=raw,file=bin/os.iso -d int -D qemu.log -no-reboot

clean:
	rm bin/*
