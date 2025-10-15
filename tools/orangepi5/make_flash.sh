#!/bin/bash

# ==============================================
# 刷写内核脚本
# 适用于 Rockchip 平台
# Usage: ./make_flash.sh [target=SD|EMMC] [uimg=path/to/uimg] # Flash Kernel to SD or eMMC
#        ./make_flash.sh [target=SD|EMMC] [uimg=path/to/uimg] rootfs=path/to/rootfs.img # Flash custom rootfs
#        ./make_flash.sh [target=SD|EMMC] partition # Flash custom partition table
# ==============================================

set -e  # 遇到错误立即退出

CURRENT_DIR=$(basename "$PWD")

UIMAGE="${CURRENT_DIR}_aarch64-opi5p.uimg"
BOOT_IMAGE="boot_sparse.img"
ORANGEPI5_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
MINILOADER="${ORANGEPI5_DIR}/MiniLoaderAll.bin"
PARTITION_TXT="${ORANGEPI5_DIR}/parameter.txt"
FLASH_TARGET=SD # 刷写目标 (SD 或 EMMC)
ROOTFS=""

# 颜色输出定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

check_device_connected() {
    check_maskrom() {
        bash -c "sudo rkdeveloptool ld" 2>/dev/null | grep -q "Maskrom"
    }

    # Wait for device to enter Maskrom mode
    RETRY_COUNT=0
    MAX_RETRIES=30  # Maximum 60 seconds (30 * 2 seconds)

    while ! check_maskrom; do
        RETRY_COUNT=$((RETRY_COUNT + 1))
        if [ $RETRY_COUNT -gt $MAX_RETRIES ]; then
            warn "Please put the Orange Pi 5 into Maskrom mode manually:"
            warn "1. Power off the device"
            warn "2. Hold the Maskrom button"
            warn "3. Connect USB-C cable"
            warn "4. Release the Maskrom button"
            error "Device not in Maskrom mode after ${MAX_RETRIES} attempts"
        fi
        
        warn "Device not in Maskrom mode (attempt $RETRY_COUNT/$MAX_RETRIES)"
        info "Waiting 2 seconds before retry..."
        sleep 2
    done
}

make_boot_image() {
    info "Creating boot image..."
    if [ ! -f "$UIMAGE" ]; then
        error "Boot image '$UIMAGE' not found! Please build it first."
    fi
    info "Found boot image: $(du -h "$UIMAGE" | cut -f1)"
    sudo bash "${ORANGEPI5_DIR}/make_boot.sh" "$UIMAGE" "$BOOT_IMAGE"
}

flash_miniloader() {
    info "Flashing Miniloader"
    info "Downloading bootloader ${MINILOADER}..."
    if timeout 4 bash -c "sudo rkdeveloptool db ${MINILOADER}"; then
        info "Bootloader downloaded successfully"
    else
        if [ $? -eq 124 ]; then
            warn "Timeout: Failed to download bootloader within 4 seconds"
            error "Restart Maskrom mode and try again"
        else
            error "Failed to download bootloader"
        fi
        exit 1
    fi
}

check_sdmmc() {
    info "Checking SD/MMC status..."
    if [ "$FLASH_TARGET" = "SD" ]; then
        info "Flashing to SD card"
        if bash -c "sudo rkdeveloptool cs 2"; then
            info "Switched to SD card successfully"
        else
            error "Failed to switch to SD card"
            exit 1
        fi
    else
        info "Flashing to eMMC"
        if bash -c "sudo rkdeveloptool cs 1"; then
            info "Switched to eMMC successfully"
        else
            error "Failed to switch to eMMC"
            exit 1
        fi
    fi
}

flash_partition() {
    info "Flashing partition table..."
    if bash -c "sudo rkdeveloptool gpt ${PARTITION_TXT}"; then
        info "Partition image flashed successfully"
        bash -c "sudo rkdeveloptool ppt"
    else
        error "Failed to flash partition image"
        exit 1
    fi
}

flash_boot_image() {
    info "Flashing boot image..."
    if bash -c "sudo rkdeveloptool wlx boot ${BOOT_IMAGE}"; then
        info "Boot image flashed successfully"
    else
        error "Failed to flash boot image"
        exit 1
    fi
}

flash_rootfs() {
    info "Flashing root filesystem..."
    if [ ! -f "$ROOTFS" ]; then
        error "Rootfs file '$ROOTFS' not found"
    fi
    if bash -c "sudo rkdeveloptool wlx root ${ROOTFS}"; then
        info "Root filesystem flashed successfully"
    else
        error "Failed to flash root filesystem"
        exit 1
    fi
}

restart_device() {
    info "Rebooting device..."
    if bash -c "sudo rkdeveloptool rd"; then
        info "Device rebooted successfully"
    else
        error "Failed to reboot device"
        exit 1
    fi
}

main() {
    echo "=========================================="
    echo " Orange Pi 5 Flashing Script"
    echo "=========================================="
    
    # 解析命令行参数
    for arg in "$@"; do
        case "$arg" in
            rootfs=*)
                ROOTFS_PATH="${arg#*=}"
                if [ ! -f "$ROOTFS_PATH" ]; then
                    error "Rootfs file '$ROOTFS_PATH' not found"
                fi
                ;;
            target=*)
                TARGET="${arg#*=}"
                if [ "$TARGET" != "SD" ] && [ "$TARGET" != "EMMC" ]; then
                    error "Invalid target '$TARGET'. Must be 'SD' or 'EMMC'"
                fi
                FLASH_TARGET="$TARGET"
                ;;
            uimg=*)
                UIMAGE="${arg#*=}"
                if [ ! -f "$UIMAGE" ]; then
                    error "Boot image '$UIMAGE' not found"
                fi
                ;;
            partition)
                PARTITION_ONLY=true
                ;;
            *)
                error "Unknown argument: $arg"
                ;;
        esac
    done

    check_device_connected
    flash_miniloader
    check_sdmmc
    

    if [ "$PARTITION_ONLY" = true ]; then
        flash_partition
        info "Partition table flashed. Exiting."
        restart_device
        exit 0
    fi

    if [ -n "$ROOTFS_PATH" ]; then
        info "Using custom rootfs: $ROOTFS_PATH"
        ROOTFS="$ROOTFS_PATH"
        flash_rootfs
    elif [ -z "$ROOTFS" ]; then
        make_boot_image
        flash_boot_image
    fi

    restart_device
    
    echo "=========================================="
    info "Flashing completed successfully!"
    echo "You can now disconnect the device."
    echo "=========================================="
}

# 执行主函数
main $@