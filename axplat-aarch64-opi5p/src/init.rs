use axplat::init::InitIf;

#[allow(unused_imports)]
use crate::config::devices::{GICD_PADDR, GICR_PADDR, TIMER_IRQ};
use crate::config::plat::PSCI_METHOD;

struct InitIfImpl;

#[impl_plat_interface]
impl InitIf for InitIfImpl {
    /// Initializes the platform at the early stage for the primary core.
    ///
    /// This function should be called immediately after the kernel has booted,
    /// and performed earliest platform configuration and initialization (e.g.,
    /// early console, clocking).
    fn init_early(_cpu_id: usize, _dtb: usize) {
        axplat::console_println!("init_early on RK3588");
        axcpu::init::init_trap();
        crate::psci::init(PSCI_METHOD);
        // Todo, compatible = "rockchip,rk3588-uart\0snps,dw-apb-uart"
        // The serial port can be used directly by default without the need for init
        // super::dw_apb_uart::init_early();

        // axplat_aarch64_peripherals::generic_timer::init_early();
        crate::generic_timer::init_early();
    }

    /// Initializes the platform at the early stage for secondary cores.
    #[cfg(feature = "smp")]
    fn init_early_secondary(_cpu_id: usize) {
        axcpu::init::init_trap();
    }

    /// Initializes the platform at the later stage for the primary core.
    ///
    /// This function should be called after the kernel has done part of its
    /// initialization (e.g, logging, memory management), and finalized the rest of
    /// platform configuration and initialization.
    fn init_later(_cpu_id: usize, _dtb: usize) {
        #[cfg(feature = "irq")]
        {
            crate::irq::init();
            crate::generic_timer::enable_irqs(TIMER_IRQ);
        }
    }

    /// Initializes the platform at the later stage for secondary cores.
    #[cfg(feature = "smp")]
    fn init_later_secondary(_cpu_id: usize) {
        #[cfg(feature = "irq")]
        {
            crate::irq::init_current_cpu();
            crate::generic_timer::enable_irqs(TIMER_IRQ);
        }
    }
}
