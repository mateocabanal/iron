pub mod gdt;
pub mod interrupts;

pub fn init() {
    gdt::init_gdt();
    interrupts::init_idt();
    x86_64::instructions::interrupts::enable();
}
