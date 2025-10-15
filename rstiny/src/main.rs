#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate log;

extern crate alloc;
extern crate axplat_aarch64_opi5p;

mod config;
mod utils;

mod test;

fn init_kernel(cpu_id: usize, arg: usize) {
    // Initialize trap, console, time.
    axplat::init::init_early(cpu_id, arg);

    // Initialize platform peripherals (not used in this example).
    axplat::init::init_later(cpu_id, arg);
}

#[axplat::main]
pub fn rust_main(cpu_id: usize, arg: usize) -> ! {
    utils::mem::clear_bss();
    init_kernel(cpu_id, arg);

    axplat::console_println!("Hello, RSTiny!");

    utils::logging::log_init();

    info!("Logging initialized. This is an info message.");

    test::run_allocator_tests();

    axplat::power::system_off()
}

#[cfg(all(target_os = "none", not(test)))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    axplat::console_println!("{info}");
    axplat::power::system_off()
}
