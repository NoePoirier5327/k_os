all: bootloader compile_kernel linkage iso

bootloader:
	# Assemblage du bootloader.
	mkdir -p bin
	nasm -f elf64 boot/src/multiboot_header.asm -o bin/multiboot_header.o
	nasm -f elf64 boot/src/long_mode_init.asm -o bin/long_mode_init.o
	nasm -f elf64 boot/src/boot.asm -o bin/boot.o

compile_kernel:	
	# Compilation du kernel.
	cargo build --release

compile_kernel_debug:
	# Compilation du kernel en mode debug
	cargo build

linkage:
	mkdir -p bin
	# Linkage du bootloader et du kernel.
	ld -n -T linker.ld -Map bin/kernel.map -o bin/kernel.bin\
		bin/multiboot_header.o\
		bin/long_mode_init.o\
		bin/boot.o\
		target/x86_64-unknown-none/release/libk_os.a

linkage_debug:
	mkdir -p bin
	# Linkage du bootloader et du kernel en mode debug.
	ld -n -T linker.ld -Map bin/kernel_debug.map -o bin/kernel_debug.bin\
		bin/multiboot_header.o\
		bin/long_mode_init.o\
		bin/boot.o\
		target/x86_64-unknown-none/debug/libk_os.a

iso:
	# Création de la structure pour GRUB
	mkdir -p bin/isofiles/boot/grub
	cp bin/kernel.bin bin/isofiles/boot/
	cp boot/grub/grub.cfg bin/isofiles/boot/grub/
	
	# Création de l'image ISO
	grub-mkrescue -o bin/k_os.iso bin/isofiles

iso_debug:
	# Création de la structure pour GRUB
	mkdir -p bin/isofiles/boot/grub
	cp bin/kernel_debug.bin bin/isofiles/boot
	cp boot/grub/grub.cfg bin/isofiles/boot/grub
	# Création de l'image iso
	grub-mkrescue -o bin/k_os_debug.iso bin/isofiles

run:
	qemu-system-x86_64 -cdrom bin/k_os.iso -no-reboot -serial stdio

debug: bootloader compile_kernel_debug linkage_debug iso_debug
	qemu-system-x86_64 -s -S -cdrom bin/k_os_debug.iso -no-reboot -serial stdio&
	gnome-terminal -- bash -c "rust-gdb bin/kernel_debug.bin -ex 'target remote :1234'"

run-log:
	qemu-system-x86_64 -drive format=raw,file=bin/k_os.iso -d int,cpu_reset -D qemu.log -no-reboot

clean:
	rm bin/*
