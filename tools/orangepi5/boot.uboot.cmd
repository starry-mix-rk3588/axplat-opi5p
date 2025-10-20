bootdev hunt ethernet
setenv ipaddr 192.168.66.103
setenv serverip 192.168.66.102
tftp 0x400000 kernel.uimg
tftp 0x300000 rk3588-orangepi-5-plus.dtb
bootm 0x400000 - 0x300000