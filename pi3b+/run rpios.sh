#!/usr/bin/env bash
qemu-system-aarch64 -M raspi3 -cpu arm1176 -m 1G -smp 4 -sd "2021-10-30-raspios-bullseye-armhf-lite.img" -dtb bcm2710-rpi-3-b-plus.dtb -kernel kernel8.img -append "rw earlyprintk loglevel=8 console=ttyAMA0,115200 dwc_otg.lpm_enable=0 root=/dev/mmcblk0p2 rootdelay=1" -serial stdio -usb -device usb-mouse -device usb-kbd
