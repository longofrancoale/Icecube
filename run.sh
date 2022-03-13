#!/bin/bash

qemu-system-x86_64 \
	-pflash OVMF.fd \
	-enable-kvm -cpu host \
	-drive file=fat:rw:sysroot/ \
	-serial stdio
