#!/bin/bash

set -xe

cd kernel
cd user-test

nasm -f elf64 main.asm
ld -Tlinker.ld main.o -o main

cd ..

cargo build --target-dir ../build/kernel --release
cd ..

mkdir -p sysroot/EFI/BOOT
cp third-party/limine/limine.sys sysroot/limine.sys
cp third-party/limine/BOOTX64.EFI sysroot/EFI/BOOT/BOOTX64.EFI
cp limine/limine.cfg sysroot/limine.cfg
cp build/kernel/x86_64-icecube/release/kernel sysroot/kernel.elf
