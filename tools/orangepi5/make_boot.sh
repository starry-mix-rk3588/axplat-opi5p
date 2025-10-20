#!/bin/bash

# ==============================================
# 创建 sparse ext4 镜像脚本
# 适用于 Rockchip 平台
# Usage: ./make_boot.sh <starry_image> <output_name> 
# ==============================================

set -e  # 遇到错误立即退出

# 判断是否有两个参数
if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <starry_image> <output_name>"
    exit 1
fi

# 配置参数
IMAGE_SIZE="100M"          # 镜像大小
MOUNT_POINT="/mnt/boot_img"  # 挂载点
OUTPUT_IMAGE="$2"  # 输出镜像文件名
KERNEL_SOURCE="$1"  # 源内核文件
TARGET_PATH="/kernel.uimg"   # 镜像中的目标路径
DTB_PATH="/rk3588-orangepi-5-plus.dtb" # 设备树文件路径
ORANGEPI5_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
BOOT_CMD="boot.cmd"

# 颜色输出定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# 检查依赖工具
check_dependencies() {
    local tools=("mkfs.ext4" "e2fsck" "resize2fs" "sudo")
    local missing=()
    
    for tool in "${tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            missing+=("$tool")
        fi
    done
    
    if [ ${#missing[@]} -ne 0 ]; then
        error "缺少必要的工具: ${missing[*]}"
    fi
    info "所有依赖工具检查通过"
}

# 清理函数
cleanup() {
    if mountpoint -q "$MOUNT_POINT"; then
        info "卸载镜像..."
        sudo umount "$MOUNT_POINT" 2>/dev/null || true
    fi
    
    if [ -d "$MOUNT_POINT" ]; then
        sudo rmdir "$MOUNT_POINT" 2>/dev/null || true
    fi
}

# 注册清理函数
trap cleanup EXIT INT TERM

# 检查源文件是否存在
check_source_file() {
    if [ ! -f "$KERNEL_SOURCE" ]; then
        error "请提供有效的内核文件路径"
    fi
    info "找到内核文件: $(du -h "$KERNEL_SOURCE" | cut -f1)"
}

# 创建 sparse 镜像
create_sparse_image() {
    info "创建 sparse ext4 镜像 (大小: $IMAGE_SIZE)..."
    
    dd if=/dev/zero of="$OUTPUT_IMAGE" bs=1 count=0 seek="$IMAGE_SIZE" status=none
    mkfs.ext4 -F "$OUTPUT_IMAGE" > /dev/null
    info "使用 dd+mkfs.ext4 创建镜像"
    
    if [ ! -f "$OUTPUT_IMAGE" ]; then
        error "创建镜像失败"
    fi
    info "镜像创建成功: $(du -h "$OUTPUT_IMAGE" | cut -f1)"
}

# 挂载并复制文件
mount_and_copy() {
    info "创建挂载点..."
    sudo mkdir -p "$MOUNT_POINT"
    
    info "挂载镜像..."
    sudo mount -o loop "$OUTPUT_IMAGE" "$MOUNT_POINT"

    info "使用 ${BOOT_CMD} 制作 boot.src 文件..."
    mkimage -A arm -T script -C none -n "TF boot" -d "${ORANGEPI5_DIR}/${BOOT_CMD}" boot.scr
    
    info "复制 boot 文件到镜像中..."
    sudo cp boot.scr "${MOUNT_POINT}"
    
    info "复制 kernel 文件到镜像中..."
    sudo cp "$KERNEL_SOURCE" "${MOUNT_POINT}${TARGET_PATH}"

    sudo cp "${ORANGEPI5_DIR}/$DTB_PATH" "${MOUNT_POINT}/rk3588-orangepi-5-plus.dtb"

    sudo ls -al "${MOUNT_POINT}"
    
    info "卸载镜像..."
    sudo umount "$MOUNT_POINT"
    sudo rmdir "$MOUNT_POINT"
    rm boot.scr
}

# 主执行流程
main() {
    echo "=========================================="
    echo "    Sparase Ext4 镜像创建与刷写工具"
    echo "=========================================="
    
    check_dependencies
    check_source_file
    
    # 清理之前的文件
    cleanup
    
    create_sparse_image
    mount_and_copy
    
    echo "=========================================="
    echo "镜像准备完成: $OUTPUT_IMAGE"
    echo "包含文件: $TARGET_PATH"
    echo "=========================================="
    
    info "镜像已保存为: $OUTPUT_IMAGE"
    info "您可以使用以下命令手动刷写:"
    info "sudo rkdeveloptool wl $FLASH_OFFSET $OUTPUT_IMAGE"
    info "sudo rkdeveloptool rd"
}

# 执行主函数
main "$@"