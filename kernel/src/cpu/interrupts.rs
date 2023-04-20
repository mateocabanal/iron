use pic8259::ChainedPics;
use spin::Lazy;
use spin::Mutex;
use spin::Once;
use x86_64::instructions::port::PortReadOnly;
use x86_64::structures::idt::InterruptDescriptorTable;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::structures::idt::PageFaultErrorCode;

use crate::println;
use crate::x2apic::LAPIC;

use crate::print;
use crate::serial_println;

pub const PIC_1_OFFSET: u8 = 32;
pub const IOAPIC_INTERRUPT_INDEX_OFFSET: u8 = 32;

static COUNT: Lazy<Mutex<i32>> = Lazy::new(|| {
    Mutex::new(0)  
});

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
    Mouse,
    ApicError,
    Syscall,
    ApicSpurious
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    idt.general_protection_fault.set_handler_fn(general_fault_handler);
    unsafe {
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(crate::cpu::gdt::DOUBLE_FAULT_IST_INDEX); // new
    }
    idt.page_fault.set_handler_fn(page_fault_handler);
    idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler); // new
    idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
    idt[InterruptIndex::ApicError.as_usize()].set_handler_fn(lapic_error);
    idt[InterruptIndex::ApicSpurious.as_usize()].set_handler_fn(spurious_interrupt);
    idt[InterruptIndex::Mouse.as_usize()].set_handler_fn(mouse_interrupt);
    idt[InterruptIndex::Syscall.as_usize()].set_handler_fn(syscall_handler);
    idt
});

pub fn init_idt() {
    IDT.load();

    //unsafe { PICS.lock().initialize() }; // new
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = PortReadOnly::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::keyboard::add_scancode(scancode);
    unsafe {
        LAPIC.try_get().unwrap().lock()
            .end_of_interrupt()
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    if let Ok(func) = crate::TIMER_FN.try_get() {
        func();
    }

    unsafe {
        LAPIC.try_get().unwrap().lock()
            .end_of_interrupt()
    }
}

extern "x86-interrupt" fn mouse_interrupt(_frame: InterruptStackFrame) {
    unsafe { LAPIC.try_get().unwrap().lock().end_of_interrupt() }
}

extern "x86-interrupt" fn syscall_handler(_frame: InterruptStackFrame) {
    log::debug!("Syscall interrupt!");
    unsafe { LAPIC.try_get().unwrap().lock().end_of_interrupt() }
}

extern "x86-interrupt" fn spurious_interrupt(_frame: InterruptStackFrame) {
    log::debug!("Received spurious interrupt!");
    unsafe { LAPIC.try_get().unwrap().lock().end_of_interrupt() }
}

extern "x86-interrupt" fn lapic_error(_frame: InterruptStackFrame) {
    println!("Local APIC error!");
    unsafe { LAPIC.try_get().unwrap().lock().end_of_interrupt() }
}


extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn general_fault_handler(stack_frame: InterruptStackFrame, some_num: u64) {
    log::error!("EXCEPTION: GENERAL FAULT\n{:#?}\n\n{}", stack_frame, some_num);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    loop {}
}
