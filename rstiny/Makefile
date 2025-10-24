# ARM RSTiny - Rust Bare Metal OS Makefile

# 项目配置
PROJECT_NAME = rstiny

TOOL_PATH = ../tools/orangepi5

kernel_elf = ../target/$(TARGET)/$(MODE)/$(PROJECT_NAME)
kernel_bin = $(kernel_elf).bin
kernel_img = $(kernel_elf).img
kernel_uimg = $(kernel_elf).uimg
kernel_asm = $(kernel_elf)_asm.txt

ifeq ($(MODE), release)
	MODE_ARG := --release
endif

# 编译选项
CARGO_FLAGS = $(MODE_ARG) --target $(TARGET)

# 默认目标
all: build

# 编译项目
build: 
	@echo "Building $(PROJECT_NAME)..."
	cargo build $(CARGO_FLAGS)
	@echo "Build completed: $(kernel_elf)"
	
	@echo "Generating $(kernel_bin)..."
	@rust-objcopy -O binary $(kernel_elf) $(kernel_bin)

	@echo "Dump $(kernel_asm)"
	@rust-objdump -d --print-imm-hex $(kernel_elf) > $(kernel_asm)

	@echo "Generating $(kernel_uimg)..."
	@mkimage -A arm64 -O linux -T kernel -C none -a 0x400000 -e 0x4000000 -n "$(PROJECT_NAME)" -d $(kernel_bin) $(kernel_uimg)

img: build
	@echo "Make $(PROJECT_NAME) on Orange Pi 5..."
	sudo bash $(TOOL_PATH)/make_boot.sh $(kernel_bin) $(kernel_img)

flash: $(kernel_bin)
	@echo "Flash $(PROJECT_NAME) to Orange Pi 5..."
	sudo bash $(TOOL_PATH)/make_flash.sh uimg=$(kernel_bin)

tftp: $(kernel_uimg)
	@echo "Copy $(PROJECT_NAME) to TFTP directory..."
	cp $(kernel_uimg) /data/docker/tftpboot/data/kernel.uimg

mini: $(kernel_uimg)
	@echo "Copy $(PROJECT_NAME) to MINI directory..."
	scp -o 'ProxyJump web@20.13.0.88' $(kernel_uimg) debin@10.1.135.123:/data/docker/tftpboot/data/