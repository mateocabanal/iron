#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

mod framebuffer;
use framebuffer::init_global_fb;

mod logger;
mod serial;
use crate::logger::init_logger;

mod cpu;
mod memory;
mod allocator;
mod task;

use x86_64::VirtAddr;

extern crate alloc;

use alloc::vec::Vec;
use alloc::boxed::Box;

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

fn kernel_main(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    let framebuffer = boot_info.framebuffer.as_mut().unwrap();
    let fb_info = framebuffer.info().clone();

    init_global_fb(framebuffer.buffer_mut(), fb_info);
    init_logger();

    println!("hello from iron_kernel");
    cpu::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    let mut mapper = unsafe { memory::init(phys_mem_offset) };

    let mut frame_allocator = unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    let mut executor = task::simple_executor::SimpleExecutor::new();
    executor.spawn(task::Task::new(example_task()));
    //executor.run();

    let mut vec: Vec<i32> = Vec::new();
    vec.push(5);

    hlt_loop();
}

async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}
