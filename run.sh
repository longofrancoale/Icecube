#!/bin/bash

qemu-system-x86_64 \
	-enable-kvm -cpu host\
	-pflash OVMF.fd \
	-drive file=fat:rw:sysroot/,format=raw \
	-serial stdio \
	-no-shutdown \
	-no-reboot
