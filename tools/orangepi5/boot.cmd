bootdev hunt ethernet
setenv ipaddr 10.19.0.107
setenv serverip 10.1.135.123
setenv netmask 255.255.254.0
setenv gatewayip 10.19.0.1
tftp 0x400000 kernel.uimg
tftp 0x300000 rk3588-orangepi-5-plus.dtb
bootm 0x400000 - 0x300000