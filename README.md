# AXPLAT 适配 rk3588（香橙派 5Plus）

文档参考：[RK 系列芯片启动流程](./tools/docs/瑞芯微%20RK%20系列芯片启动流程简析.pdf)
根据启动流程， 建议将 U-Boot 烧写在 SPI NOR FLASH，系统镜像烧写在 SD 卡/eMMC

## 安装 rkdeveloptool 工具

参考文献：[rkdeveloptool](https://docs.radxa.com/rock3/rock3c/low-level-dev/rkdeveloptool?host-os=archlinux)

```bash
sudo apt-get update
sudo apt-get install -y libudev-dev libusb-1.0-0-dev dh-autoreconf pkg-config libusb-1.0 build-essential git wget
git clone https://github.com/rockchip-linux/rkdeveloptool
cd rkdeveloptool
autoreconf -i
./configure
make -j $(nproc)
sudo cp rkdeveloptool /usr/local/sbin/
```

## 香橙派进入 Maskrom 模式

使用数据线连接香橙派，按住 Maskrom 键，然后重新上电就可以，每次烧写都需要手动进入该模式

```bash
sudo rkdeveloptool ld
# DevNo=1 Vid=0x2207,Pid=0x350b,LocationID=103    Maskrom
```

## 烧写 MiniLoaderAll.bin

每次烧写都需要先烧写 MiniLoaderAll.bin，这里是启动烧写流程必须的驱动程序，识别 SD 卡，EMMC 芯片都需要它，并且在每次断电都会消失

``` bash
sudo rkdeveloptool db MiniLoaderAll.bin # 烧写 MiniLoaderAll.bin
```

## 烧写 U-Boot (仅第一次)

烧写完 MiniLoaderAll.bin 以后，就可以识别出 SPI NOR FLASH 了，使用命令

```bash
sudo rkdeveloptool cs 9 # 切换到 SPI NOR FLASH 模式 [storage: 1=EMMC, 2=SD, 9=SPINOR]
sudo rkdeveloptool wl 0 u-boot-orangepi5-plus-spi.bin # 烧写 U-Boot
```

## 烧写 SDMMC / eMMC

因为 U-Boot 的自动启动命令与 SD 卡的格式以及分区有关，所以这里直接制作了完整的 img 镜像，使用脚本 `./tools/orangepi5/make_boot.sh` 可以自动生成 SD 卡镜像以及 `boot.scr` 启动脚本。

```bash
bash ./tools/orangepi5/make_boot.sh <KERNEL_IMAGE> <IMG_FILE_NAME> 
```

切换到对应的存储设备, 如 sd 卡
``` bash
sudo rkdeveloptool cs 2 # 切换到SD卡模式 [storage: 1=EMMC, 2=SD, 9=SPINOR]
```

创建分区表 (仅需要格式化硬盘的时候)

如果你的存储设备里面没有初始化 GPT 分区表，那么就需要手动创建分区表
``` bash
sudo rkdeveloptool gpt tools/orangepi5/parameter.txt 
sudo rkdeveloptool ppt # 打印 GPT 分区表
```

将镜像烧写进去，使用命令

```bash
sudo rkdeveloptool wlx boot <IMG_FILE_NAME> # 直接烧写SD卡镜像
```

## 重启香橙派

```bash
sudo rkdeveloptool rd
```

## Rkdeveloptool

``` bash
dd if=/dev/zero of=zeros.img bs=1M count=128 # 生成空文件
```

## U-Boot 操作

``` bash
mmc list # 查看设备
mmc dev 1 # 切换到指定设备
mmc part # 查看分区表
ext4load mmc 1:1 0x400000 kernel.uimg # 加载内核
go 0x400000 # 跳转到指定位置
```
