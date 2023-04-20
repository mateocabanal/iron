pub mod gdt;
pub mod interrupts;

pub fn init() {
    gdt::init_gdt();
    log::debug!("init'd gdt");
    interrupts::init_idt();
    log::debug!("init'd idt");
//    x86_64::instructions::interrupts::enable();
    log::debug!("enabled interrupts");
}
