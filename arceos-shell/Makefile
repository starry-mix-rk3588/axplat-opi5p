# Arceos Shell Makefile

PROJECT_NAME = arceos-shell

TOOL_PATH = $(PWD)/tools/orangepi5
PLAT_NAME = axplat-aarch64-opi5p
PLAT_CONFIG = $(PWD)/axplat-aarch64-opi5p/axconfig.toml

kernel_elf = $(PWD)/target/$(TARGET)/$(MODE)/$(PROJECT_NAME)
kernel_bin = $(kernel_elf).bin
kernel_img = $(kernel_elf).img
kernel_asm = $(kernel_elf)_asm.txt

ifeq ($(MODE), release)
	MODE_ARG := --release
endif

FEATURE_FLAGS = "axstd/myplat axstd/log-level-warn axstd/bus-mmio axstd/page-alloc-4g axstd/driver-ramdisk axstd/smp"

# 编译选项
CARGO_FLAGS = $(MODE_ARG) --target $(TARGET) --features $(FEATURE_FLAGS)

OUT_CONFIG = $(PWD)/.axconfig.toml

CONFIG_ARGS := \
  $(PWD)/$(APP)/defconfig.toml $(PLAT_CONFIG) \
  -w 'arch="$(ARCH)"' \
  -w 'platform="$(PLAT_NAME)"' \
  -o "$(OUT_CONFIG)"

export AX_ARCH = $(ARCH)
export AX_PLATFORM = $(PLAT_NAME)
export AX_MODE=$(MODE)
export AX_LOG=$(LOG)
export AX_TARGET=$(TARGET)
export AX_CONFIG_PATH=$(OUT_CONFIG)

# 默认目标
all: build

$(OUT_CONFIG): 
	echo "Generating configuration file at $(OUT_CONFIG)..."
	axconfig-gen $(CONFIG_ARGS)

# 编译项目
build: $(OUT_CONFIG)
	@echo "Building $(PROJECT_NAME)..."
	cargo build $(CARGO_FLAGS)
	@echo "Build completed: $(kernel_elf)"
	
	@echo "Generating $(kernel_bin)..."
	@rust-objcopy -O binary $(kernel_elf) $(kernel_bin)

	@echo "Dump $(kernel_asm)"
	@rust-objdump -d --print-imm-hex $(kernel_elf) > $(kernel_asm)

img: build
	@echo "Make $(PROJECT_NAME) on Orange Pi 5..."
	sudo bash $(TOOL_PATH)/make_boot.sh $(kernel_bin) $(kernel_img)

$(kernel_img): img

flash: $(kernel_img)
	@echo "Flash $(PROJECT_NAME) to Orange Pi 5..."
	sudo bash $(TOOL_PATH)/make_flash.sh uimg=$(kernel_img)

upload: $(kernel_img)
	@echo "Upload $(PROJECT_NAME) to Orange Pi 5..."
	bash $(TOOL_PATH)/upload_flash.sh $(kernel_img)