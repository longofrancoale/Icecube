#!/bin/bash

qemu-system-x86_64 \
	-d int \
	-pflash OVMF.fd \
	-drive file=fat:rw:sysroot/,format=raw \
	-monitor stdio \
	-no-shutdown \
	-no-reboot \
# 	-enable-kvm -cpu host \

