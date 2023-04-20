#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

mod framebuffer;
use bootloader_api::BootInfo;
use framebuffer::init_global_fb;

mod logger;
mod serial;
use crate::logger::init_logger;

mod cpu;
mod memory;
mod allocator;
mod task;
mod x2apic;
mod acpi;
mod keyboard;

extern crate alloc;

use alloc::vec::Vec;

const CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(bootloader_api::config::Mapping::Dynamic);
    config.kernel_stack_size = 100 * 1024; // 100 KiB
    config
};

bootloader_api::entry_point!(kernel_main, config = &CONFIG);

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::error!("{info}");
    hlt_loop();
}

fn init(boot_info: &'static BootInfo) {
    init_global_fb(boot_info);
    init_logger();
    memory::init(boot_info);
    allocator::init_heap();
    let apic = acpi::init(boot_info);
    x2apic::init(&apic);
    cpu::init();
}

fn kernel_main(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    init(boot_info);
    println!("init'd cpu");
    println!("hello from iron_kernel");


    let mut vec: Vec<i32> = Vec::new();
    vec.push(5);

    println!("DONE");
    
    x86_64::instructions::interrupts::enable();

    let mut executor = task::executor::Executor::new();
    executor.spawn(task::Task::new(keyboard::print_keypresses()));
    executor.run();

    hlt_loop();
}

async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}
