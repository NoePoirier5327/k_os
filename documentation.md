# The KOs project
KOs is an exotic rust operating system.

## The syscalls
The kernel syscalls are listed [here](https://github.com/NoePoirier5327/k_os/blob/master/syscalls.csv).<br>
You can call them using software interruptions in 32 bit applications and using the syscall instructions in 64 bit applications.<br>
Unlike UNIX like kernel, the console writing syscalls and file writing syscalls are separate.

## The file system
The file system will be a mix of LINUX and WINDOWS solutions. Each files will have a reading and writing right corresponding to the acreditation level of the current user but no one will have the right to set a file as executable.<br>
In fact, there will be a dedicated way of making a file executable and it will be up to the compilers to make it possible. Thus, it will prevent problems like multiple file extensions or multiple files type that have the right to be executable as applications.<br>
About the disk architexture, KOs will use an alphabetical system like windows but with a total access to the unmounted detected periphericals like `/dev/` in linux.

## About the UI
Like linux, the idea is to have multiples GUI environnements avalaibles open to rice. However, unlike linux, there will be an official GUI environnement avalaible for new users.<br>
A CLI version will also be available for installation.

## The applications
To install user applications on the os, it is intended to get the code of open-source projects and compile them by hand for now.<br>
In the future, to make it easier, there might be a packet manager like yay.<br>
There will also be some official systems applications deletable at any moments if you don't like them.
