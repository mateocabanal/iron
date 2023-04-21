use acpi::platform::interrupt::Apic;
use conquer_once::spin::OnceCell;
use spin::Mutex;
use x2apic::ioapic::{IoApic, IrqFlags, IrqMode, RedirectionTableEntry};
use x2apic::lapic::{xapic_base, LocalApic, LocalApicBuilder, TimerDivide};
use x86_64::instructions::port::Port;
use x86_64::VirtAddr;

use crate::cpu::interrupts::InterruptIndex;
use crate::{hlt_loop, println};

pub static LAPIC: OnceCell<Mutex<LocalApic>> = OnceCell::uninit();
pub static IOAPIC: OnceCell<Mutex<IoApic>> = OnceCell::uninit();

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum IrqVector {
    Keyboard = 1,
    Mouse = 12,
}

unsafe fn disable_pic() {
    Port::<u8>::new(0xa1).write(0xff);
    Port::<u8>::new(0x21).write(0xff);
}

pub fn init(apic: &Apic) {
    unsafe { disable_pic() }
    init_lapic(apic);
    unsafe {
        init_ioapic(apic);
    }
}

pub fn init_lapic(apic: &Apic) {
    let apic_phys_addr = apic.local_apic_address;
    let apic_virt_addr = crate::memory::PHYS_MEM_OFFSET.get().unwrap().as_u64() + apic_phys_addr;
    crate::map_physical_to_virtual!(apic_phys_addr, apic_virt_addr);

    log::trace!("mapped phys addr to virt addr");

    let lapic = LocalApicBuilder::new()
        .spurious_vector(InterruptIndex::ApicSpurious as usize)
        .timer_vector(InterruptIndex::Timer as usize)
        .error_vector(InterruptIndex::ApicError as usize)
        .timer_divide(TimerDivide::Div2)
        .timer_initial(100_000_000)
        .set_xapic_base(apic_virt_addr)
        .build();

    if let Ok(mut lapic) = lapic {
        log::debug!("lapic built successfully!");

        // lapic.version() crashes os!!!
        // log::debug!("LAPIC VERSION: {}", unsafe { lapic.version() });
        unsafe {
            lapic.enable();
        }

        LAPIC.init_once(|| Mutex::new(lapic));
    } else {
        log::error!("lapic failed to build");
        hlt_loop();
    }
}

unsafe fn init_ioapic(apic: &Apic) {
    let physical_address = apic.io_apics.get(0).unwrap().address as u64;
    let phys_mem_offset = crate::memory::PHYS_MEM_OFFSET.try_get().unwrap();
    let virtual_address = phys_mem_offset.as_u64() + physical_address;
    crate::map_physical_to_virtual!(physical_address, virtual_address);

    let mut ioapic = IoApic::new(virtual_address);
    ioapic.init(crate::cpu::interrupts::IOAPIC_INTERRUPT_INDEX_OFFSET);
    IOAPIC.init_once(|| Mutex::new(ioapic));

    ioapic_add_entry(IrqVector::Keyboard, InterruptIndex::Keyboard);
    ioapic_add_entry(IrqVector::Mouse, InterruptIndex::Mouse);
}

unsafe fn ioapic_add_entry(irq: IrqVector, vector: InterruptIndex) {
    let lapic = LAPIC.try_get().unwrap().lock();
    let mut io_apic = IOAPIC.try_get().unwrap().lock();
    let mut entry = RedirectionTableEntry::default();
    entry.set_mode(IrqMode::Fixed);
    entry.set_dest(lapic.id() as u8);
    entry.set_vector(vector as u8);
    entry.set_flags(IrqFlags::LEVEL_TRIGGERED | IrqFlags::LOW_ACTIVE | IrqFlags::MASKED);
    io_apic.set_table_entry(irq as u8, entry);
    io_apic.enable_irq(irq as u8);
}
