//! Rust 版 DW8250 UART 驱动，完整按 C 版 `t_uart_dwp.c` 逻辑改写

#![allow(dead_code)]

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use core::ptr::{read_volatile, write_volatile};
use core::arch::asm;
use kspin::SpinNoIrq;

const UART_BASE: usize = 0xfeb5_0000;
const UART_IRQ: u32 = 333 + 32; // 365

const RBR: usize = 0x00; // 读
const THR: usize = 0x00; // 写
const IER: usize = 0x04;
const IIR: usize = 0x08; // 读
const FCR: usize = 0x08; // 写
const LCR: usize = 0x0c;
const MCR: usize = 0x10;
const LSR: usize = 0x14;
const MSR: usize = 0x18;
const SCR: usize = 0x1c;
const USR: usize = 0x7c; // DW 定义
const DLL: usize = 0x00; // DLAB=1 时访问
const DLM: usize = 0x04; // DLAB=1 时访问

// LSR bits
const LSR_DR: u32 = 1 << 0;   // Data Ready
const LSR_THRE: u32 = 1 << 5; // Transmit Holding Register Empty
const LSR_TEMT: u32 = 1 << 6; // Transmitter Empty (THR & TSR)

// IER bits
const IER_RDI: u32 = 1 << 0;  // RX Available
const IER_THRI: u32 = 1 << 1; // TX Holding Register Empty

// FCR bits
const FCR_ENABLE_FIFO: u32 = 1 << 0;
const FCR_CLEAR_RCVR: u32 = 1 << 1;
const FCR_CLEAR_XMIT: u32 = 1 << 2;

// LCR bits
const LCR_DLAB: u32 = 1 << 7;

// USR bits (DW)
const USR_BUSY: u32 = 1 << 0;

// 缓冲区大小与 C 版一致
const TX_BUF_SIZE: usize = 1024;
const RX_BUF_SIZE: usize = 1024;

#[inline(always)]
fn mmio_read32(addr: usize) -> u32 { unsafe { read_volatile(addr as *const u32) } }
#[inline(always)]
fn mmio_write32(val: u32, addr: usize) { unsafe { write_volatile(addr as *mut u32, val) } }

#[inline(always)]
fn nop() { unsafe { asm!("nop", options(nomem, nostack, preserves_flags)) } }

#[inline(always)]
fn wfi() { unsafe { asm!("wfi", options(nomem, nostack, preserves_flags)) } }

/// 简单环形缓冲区
struct RingBuffer<const N: usize> {
    buf: [u8; N],
    head: usize,
    tail: usize,
    count: usize,
}

impl<const N: usize> RingBuffer<N> {
    const fn new() -> Self {
        Self { buf: [0; N], head: 0, tail: 0, count: 0 }
    }
    #[inline] fn is_empty(&self) -> bool { self.count == 0 }
    #[inline] fn is_full(&self) -> bool { self.count >= N }
    #[inline] fn put(&mut self, c: u8) -> bool {
        if self.is_full() { return false; }
        self.buf[self.head] = c;
        self.head = (self.head + 1) % N;
        self.count += 1;
        true
    }
    #[inline] fn get(&mut self) -> Option<u8> {
        if self.is_empty() { return None; }
        let c = self.buf[self.tail];
        self.tail = (self.tail + 1) % N;
        self.count -= 1;
        Some(c)
    }
}

/// DW8250 UART 驱动（对应单例）
pub struct DW8250 {
    base: usize,
    tx: UnsafeCell<RingBuffer<TX_BUF_SIZE>>,
    rx: UnsafeCell<RingBuffer<RX_BUF_SIZE>>,
    tx_lock: SpinNoIrq<()>, // 进入/退出时禁用/恢复 IRQ
    rx_lock: SpinNoIrq<()>,
    initialized: AtomicBool,
    // 调试计数
    tx_irq_count: AtomicU32,
    rx_irq_count: AtomicU32,
    last_iir_value: AtomicU32,
    tx_sent_total: AtomicU32,
}

// 安全性：
// - 通过 SpinNoIrq 保证中断上下文与普通上下文的并发安全，内部可 UnsafeCell 可变访问
unsafe impl Sync for DW8250 {}

impl DW8250 {
    pub const fn new(base_vaddr: usize) -> Self {
        Self {
            base: base_vaddr,
            tx: UnsafeCell::new(RingBuffer::new()),
            rx: UnsafeCell::new(RingBuffer::new()),
            tx_lock: SpinNoIrq::new(()),
            rx_lock: SpinNoIrq::new(()),
            initialized: AtomicBool::new(false),
            tx_irq_count: AtomicU32::new(0),
            rx_irq_count: AtomicU32::new(0),
            last_iir_value: AtomicU32::new(0),
            tx_sent_total: AtomicU32::new(0),
        }
    }

    #[inline(always)]
    fn reg(&self, off: usize) -> usize { self.base + off }

    #[inline(always)]
    fn read(&self, off: usize) -> u32 { mmio_read32(self.reg(off)) }
    #[inline(always)]
    fn write(&self, off: usize, val: u32) { mmio_write32(val, self.reg(off)) }

    #[inline]
    fn tx_ready(&self) -> bool { (self.read(LSR) & LSR_THRE) != 0 }
    #[inline]
    fn rx_ready(&self) -> bool { (self.read(LSR) & LSR_DR) != 0 }

    #[inline]
    fn enable_tx_interrupt(&self) {
        let mut ier = self.read(IER);
        ier |= IER_THRI;
        self.write(IER, ier);
    }

    #[inline]
    fn disable_tx_interrupt(&self) {
        let mut ier = self.read(IER);
        ier &= !IER_THRI;
        self.write(IER, ier);
    }

    #[inline]
    fn enable_rx_interrupt(&self) {
        let mut ier = self.read(IER);
        ier |= IER_RDI;
        self.write(IER, ier);
    }

    /// 等待 UART 完全空闲（USR.BUSY=0 且 LSR.TEMT=1）
    fn wait_idle(&self) {
        let mut timeout = 100_000i32;
        while timeout > 0 {
            let usr = self.read(USR);
            let lsr = self.read(LSR);
            if (usr & USR_BUSY) == 0 && (lsr & LSR_TEMT) != 0 { return; }
            nop();
            timeout -= 1;
        }
        // 超时也返回，避免卡死
    }

    /// 早期初始化（无中断）
    pub fn early_init(&self) {
        self.wait_idle();
        // 关中断
        self.write(IER, 0);
        // 设置波特率：24MHz / 16 / 1 = 1_500_000
        let lcr0 = self.read(LCR);
        self.write(LCR, lcr0 | LCR_DLAB);
        self.write(DLL, 1);
        self.write(DLM, 0);
        self.write(LCR, lcr0 & !LCR_DLAB);
        // 8N1
        self.write(LCR, 0x3);
        // 使能 FIFO 并清空
        self.write(FCR, FCR_ENABLE_FIFO | FCR_CLEAR_RCVR | FCR_CLEAR_XMIT);
    }

    /// 完整初始化（中断+缓冲）
    pub fn init(&self) {
        if self.initialized.load(Ordering::Acquire) { return; }

        // 复位缓冲
        {
            let _g = self.tx_lock.lock();
            unsafe { *self.tx.get() = RingBuffer::new(); }
        }
        {
            let _g = self.rx_lock.lock();
            unsafe { *self.rx.get() = RingBuffer::new(); }
        }

        self.wait_idle();
        self.write(IER, 0);

        let lcr0 = self.read(LCR);
        self.write(LCR, lcr0 | LCR_DLAB);
        self.write(DLL, 1);
        self.write(DLM, 0);
        self.write(LCR, lcr0 & !LCR_DLAB);
        self.write(LCR, 0x3); // 8N1
        self.write(FCR, FCR_ENABLE_FIFO | FCR_CLEAR_RCVR | FCR_CLEAR_XMIT);

        // 安装中断处理和 GIC 配置交给外部 C 侧（保持与 C 一致的符号）
        // unsafe {
        //     irq_install(UART_IRQ, dw_uart_interrupt_handler);
        //     gicv3_set_int_trigger(UART_IRQ, 0); // level
        //     gicv3_set_int_target(UART_IRQ, 0x1); // CPU0
        //     gicv3_enable_int(UART_IRQ, true);
        // }

        self.enable_rx_interrupt();

        self.initialized.store(true, Ordering::Release);
    }

    /// 非阻塞发送一个字符（已初始化才有效）
    pub fn putchar_nb(&self, c: u8) -> bool {
        if !self.initialized.load(Ordering::Acquire) { return false; }
        let mut need_tx_int = false;

        let _g = self.tx_lock.lock();
        // 换行处理：先压入 '\r'
        if c == b'\n' {
            if unsafe { (&mut *self.tx.get()).put(b'\r') } {
                need_tx_int = true;
            } else {
                return false; // 缓冲区满
            }
        }
        let ok = unsafe { (&mut *self.tx.get()).put(c) };
        if ok { need_tx_int = true; }
        drop(_g);

        if need_tx_int { self.enable_tx_interrupt(); }
        ok
    }

    /// 非阻塞接收一个字符（已初始化）
    pub fn getchar_nb(&self) -> Option<u8> {
        if !self.initialized.load(Ordering::Acquire) { return None; }
        let _g = self.rx_lock.lock();
        let v = unsafe { (&mut *self.rx.get()).get() };
        drop(_g);
        v
    }

    /// 阻塞发送（未初始化则直接轮询硬件；已初始化则尝试若干次）
    pub fn putchar(&self, c: u8) {
        if !self.initialized.load(Ordering::Acquire) {
            // Early：直接写寄存器
            if c == b'\n' {
                while (self.read(LSR) & LSR_THRE) == 0 { nop(); }
                self.write(THR, b'\r' as u32);
            }
            while (self.read(LSR) & LSR_THRE) == 0 { nop(); }
            self.write(THR, c as u32);
            return;
        }

        if self_putchar_nb(self, c) { return; }
        // 缓冲区满，短暂重试
        let mut retry = 0;
        while retry < 10_000 {
            if self_putchar_nb(self, c) { return; }
            // 让中断有时间把数据搬走
            let mut i = 0; while i < 100 { nop(); i += 1; }
            retry += 1;
        }
        // 超时丢弃（C 版会 warn，这里略过日志）
    }

    /// 阻塞接收一个字符
    pub fn getchar(&self) -> Option<u8> {
        if self.initialized.load(Ordering::Acquire) {
            loop {
                if let Some(c) = self.getchar_nb() { return Some(c); }
                wfi();
            }
        } else {
            // Early：直接检查硬件，如果没有数据则返回 None
            // 这里不太明白上层是怎么写的？ 上层有忙等逻辑吗？
            if (self.read(LSR) & LSR_DR) != 0 {
                Some((self.read(RBR) & 0xFF) as u8)
            } else {
                None
            }
        }
    }

    /// 刷新：等待软件缓冲清空，再等硬件 TEMT
    pub fn flush(&self) {
        if !self.initialized.load(Ordering::Acquire) { return; }

        // 第一阶段：等待软件缓冲区为空
        let mut timeout = 100_000i32;
        while timeout > 0 {
            let empty = {
                let _g = self.tx_lock.lock();
                let empty = unsafe { (&*self.tx.get()).is_empty() };
                drop(_g);
                empty
            };
            if empty { break; }
            let mut i = 0; while i < 10 { nop(); i += 1; }
            timeout -= 1;
        }

        // 第二阶段：等待硬件完全空
        let mut timeout2 = 10_000i32;
        while timeout2 > 0 {
            if (self.read(LSR) & LSR_TEMT) != 0 { break; }
            let mut i = 0; while i < 10 { nop(); i += 1; }
            timeout2 -= 1;
        }
    }

    /// IRQ 处理（与 C 版一致）
    pub fn handle_irq(&self) {
        let iir = self.read(IIR) & 0xF;
        self.last_iir_value.store(iir, Ordering::Relaxed);

        // RX 可读：0x4 或 0xC
        if iir == 0x4 || iir == 0xC {
            self.rx_irq_count.fetch_add(1, Ordering::Relaxed);
            let _g = self.rx_lock.lock();
            while self.rx_ready() {
                let c = (self.read(RBR) & 0xFF) as u8;
                let _ = unsafe { (&mut *self.rx.get()).put(c) };
            }
            drop(_g);
        }

        // TX 空或 Busy Detect：0x2 或 0x7
        if iir == 0x2 || iir == 0x7 {
            if iir == 0x7 { let _ = self.read(USR); } // 读 USR 清 busy detect
            self.tx_irq_count.fetch_add(1, Ordering::Relaxed);

            let _g = self.tx_lock.lock();
            let mut sent = 0;
            const MAX_BATCH: usize = 16;
            while sent < MAX_BATCH {
                if let Some(c) = unsafe { (&mut *self.tx.get()).get() } {
                    self.write(THR, c as u32);
                    sent += 1;
                    self.tx_sent_total.fetch_add(1, Ordering::Relaxed);
                } else {
                    break;
                }
            }
            let empty = unsafe { (&*self.tx.get()).is_empty() };
            drop(_g);
            if empty { self.disable_tx_interrupt(); }
        }
    }

    // Debug/状态查询
    pub fn rx_available(&self) -> bool {
        if !self.initialized.load(Ordering::Acquire) { return false; }
        let _g = self.rx_lock.lock();
        let available = !unsafe { (&*self.rx.get()).is_empty() };
        drop(_g);
        available
    }
    pub fn tx_buffer_usage(&self) -> u32 {
        if !self.initialized.load(Ordering::Acquire) { return 0; }
        let _g = self.tx_lock.lock();
        let usage = unsafe { (&*self.tx.get()).count as u32 };
        drop(_g);
        usage
    }
    pub fn get_stats(&self) -> (u32, u32, u32, u32) {
        let tx_irqs = self.tx_irq_count.load(Ordering::Relaxed);
        let rx_irqs = self.rx_irq_count.load(Ordering::Relaxed);
        let tx_usage = self.tx_buffer_usage();
        let rx_usage = {
            let _g = self.rx_lock.lock();
            let c = unsafe { (&*self.rx.get()).count as u32 };
            drop(_g);
            c
        };
        (tx_irqs, rx_irqs, tx_usage, rx_usage)
    }
    pub fn is_tx_interrupt_enabled(&self) -> bool { (self.read(IER) & IER_THRI) != 0 }
    pub fn last_iir(&self) -> u32 { self.last_iir_value.load(Ordering::Relaxed) }
    pub fn tx_sent_total(&self) -> u32 { self.tx_sent_total.load(Ordering::Relaxed) }
}

#[inline(always)]
fn self_putchar_nb(uart: &DW8250, c: u8) -> bool { uart_putchar_nb_shim(uart, c) }

#[inline(always)]
fn uart_putchar_nb_shim(uart: &DW8250, c: u8) -> bool { uart.putchar_nb(c) }
