///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use conquer_once::spin::OnceCell;
use x86_64::instructions::port::Port;
use pic8259::ChainedPics;
use x2apic::lapic::{LocalApic, LocalApicBuilder};
use x2apic::ioapic::{IoApic, IrqFlags};
use acpi_crate::InterruptModel;
use crate::acpi::ACPI_TABLES;
use crate::{both_println, PHYS_MEM_OFFSET};
use crate::util::halt_loop;


pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

// TODO: make a non-synchronized, constant, static bool for whether
// ASICs are present or not. Unsafe set via *const to *mut once
// in the interrupt controller init
pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

pub static LOCAL_APIC: spin::Mutex<Option<LocalApic>> = spin::Mutex::new(None);
pub static IO_APIC: spin::Mutex<Option<IoApic>> = spin::Mutex::new(None);

static IDT: OnceCell<InterruptDescriptorTable> = OnceCell::uninit();

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = 32,
    Keyboard,
    Cascade,
    Com2,
    Com1,
    LPT2,
    FloppyDisk,
    LPT1,
    CMOS,
    Peripheral1,
    Peripheral2,
    Peripheral3,
    PS2Mouse,
    Coprocessor,
    PrimaryATA,
    SecondaryATA,
}

impl InterruptIndex {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub fn early_init_interrupts() {
    let mut step = crate::StartupStep::begin("Initializing IDT");
    IDT.try_init_once(|| {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(crate::arch::gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.non_maskable_interrupt.set_handler_fn(non_maskable_interrupt_handler);
        idt.alignment_check.set_handler_fn(alignment_check_handler);
        idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        idt.device_not_available.set_handler_fn(device_not_available_handler);
        idt.divide_error.set_handler_fn(divide_error_handler);
        idt.security_exception.set_handler_fn(security_exception_handler);
        idt.simd_floating_point.set_handler_fn(simd_floating_point_handler);
        idt.x87_floating_point.set_handler_fn(x87_floating_point_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
        idt.invalid_tss.set_handler_fn(invalid_tss_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.machine_check.set_handler_fn(machine_check_handler);
        idt.overflow.set_handler_fn(overflow_handler);

        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::Cascade.as_usize()].set_handler_fn(cascade_interrupt_handler);
        idt[InterruptIndex::Com2.as_usize()].set_handler_fn(com2_interrupt_handler);
        idt[InterruptIndex::Com1.as_usize()].set_handler_fn(com1_interrupt_handler);
        idt[InterruptIndex::LPT2.as_usize()].set_handler_fn(lpt2_interrupt_handler);
        idt[InterruptIndex::FloppyDisk.as_usize()].set_handler_fn(floppy_interrupt_handler);
        idt[InterruptIndex::LPT1.as_usize()].set_handler_fn(lpt1_interrupt_handler);
        idt[InterruptIndex::CMOS.as_usize()].set_handler_fn(cmos_interrupt_handler);
        idt[InterruptIndex::Peripheral1.as_usize()].set_handler_fn(disk_irq_handler);
        idt[InterruptIndex::Peripheral2.as_usize()].set_handler_fn(disk_irq_handler);
        idt[InterruptIndex::Peripheral3.as_usize()].set_handler_fn(disk_irq_handler);
        idt[InterruptIndex::PS2Mouse.as_usize()].set_handler_fn(ps2_mouse_interrupt_handler);
        idt[InterruptIndex::Coprocessor.as_usize()].set_handler_fn(coprocessor_interrupt_handler);
        idt[InterruptIndex::PrimaryATA.as_usize()].set_handler_fn(disk_irq_handler);
        idt[InterruptIndex::SecondaryATA.as_usize()].set_handler_fn(disk_irq_handler);
        idt
    }).expect("early_init_interrupts should only be called once");
    IDT.get().unwrap().load();
    step.ok();
}

pub fn late_init_interrupts() {
    crate::both_print!("Initializing interrupt controllers...");
    unsafe { PICS.lock().initialize() };
    crate::both_print!(" PICs configured...");
    let interrupt_model = ACPI_TABLES.get().unwrap().platform_info().unwrap().interrupt_model;
    if let InterruptModel::Apic(a) = interrupt_model {
        unsafe {
            let ioapic_addr = a.io_apics[0].address;
            let mut ioapic = x2apic::ioapic::IoApic::new(ioapic_addr as u64 + PHYS_MEM_OFFSET);
            ioapic.init(32);

            let mut entry = x2apic::ioapic::RedirectionTableEntry::default();
            entry.set_mode(x2apic::ioapic::IrqMode::External);
            entry.set_flags(IrqFlags::LEVEL_TRIGGERED | IrqFlags::LOW_ACTIVE | IrqFlags::MASKED);
            entry.set_dest(0); // CPU(s)
            ioapic.set_table_entry(crate::arch::interrupts::InterruptIndex::Keyboard.as_u8(), entry);
            ioapic.enable_irq(crate::arch::interrupts::InterruptIndex::Keyboard.as_u8()-32);

            *IO_APIC.lock() = Some(ioapic);
        }
        let mut lapic = LocalApicBuilder::new()
            .timer_vector(crate::arch::interrupts::InterruptIndex::Timer.as_usize())
            .error_vector(crate::arch::interrupts::InterruptIndex::Cascade.as_usize())
            .spurious_vector(0xFF)
            .set_xapic_base(a.local_apic_address + PHYS_MEM_OFFSET)
            .build()
            .unwrap_or_else(|err| panic!("{}", err));
        unsafe { lapic.enable(); }
        *LOCAL_APIC.lock() = Some(lapic);

        crate::both_println!(" APICs configured.");
    }
    else {
        crate::both_println!(" No APIC found. Using PICs as fallback.");
    }

    x86_64::instructions::interrupts::enable();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    both_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, e: u64) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}\n{}", stack_frame, e);
}

extern "x86-interrupt" fn page_fault_handler(frame: InterruptStackFrame, err: PageFaultErrorCode) {
    use x86_64::registers::control::Cr2;

    both_println!("EXCEPTION: PAGE FAULT");
    both_println!("Accessed Address: {:?}", Cr2::read());
    both_println!("Error Code: {:?}", err);
    both_println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn non_maskable_interrupt_handler(frame: InterruptStackFrame) {
    both_println!("EXCEPTION: NMI");
    both_println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn alignment_check_handler(frame: InterruptStackFrame, error_code: u64) {
    both_println!("EXCEPTION: ALIGNMENT CHECK");
    both_println!("Error code: {:X}", error_code);
    both_println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn bound_range_exceeded_handler(frame: InterruptStackFrame) {
    both_println!("EXCEPTION: BOUND RANGE EXCEEDED");
    both_println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn segment_not_present_handler(frame: InterruptStackFrame, error_code: u64) {
    both_println!("EXCEPTION: SEGMENT NOT PRESENT");
    both_println!("Error code: {:X}", error_code);
    both_println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn general_protection_fault_handler(frame: InterruptStackFrame, error_code: u64) {
    both_println!("EXCEPTION: GENERAL PROTECTION FAULT");
    both_println!("Error code: {:X}", error_code);
    both_println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn device_not_available_handler(frame: InterruptStackFrame) {
    both_println!("EXCEPTION: DEVICE NOT PRESENT");
    both_println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn divide_error_handler(frame: InterruptStackFrame) {
    both_println!("EXCEPTION: DIVIDE ERROR");
    both_println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn security_exception_handler(frame: InterruptStackFrame, error_code: u64) {
    both_println!("EXCEPTION: SECURITY EXCEPTION");
    both_println!("Error code: {:X}", error_code);
    both_println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn simd_floating_point_handler(frame: InterruptStackFrame) {
    both_println!("EXCEPTION: SIMD FLOATING POINT ERROR");
    both_println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn x87_floating_point_handler(frame: InterruptStackFrame) {
    both_println!("EXCEPTION: X87 FLOATING POINT ERROR");
    both_println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn stack_segment_fault_handler(frame: InterruptStackFrame, error_code: u64) {
    both_println!("EXCEPTION: STACK SEGMENT FAULT HANDLER");
    both_println!("Error code: {:X}", error_code);
    both_println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn invalid_tss_handler(frame: InterruptStackFrame, error_code: u64) {
    both_println!("EXCEPTION: INVALID TSS");
    both_println!("Error code: {:X}", error_code);
    both_println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn invalid_opcode_handler(frame: InterruptStackFrame) {
    both_println!("EXCEPTION: INVALID OPCODE");
    both_println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn machine_check_handler(frame: InterruptStackFrame) -> ! {
    both_println!("EXCEPTION: MACHINE CHECK");
    both_println!("{:#?}", frame);
    halt_loop()
}

extern "x86-interrupt" fn overflow_handler(frame: InterruptStackFrame) {
    both_println!("EXCEPTION: OVERFLOW");
    both_println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn disk_irq_handler(_frame: InterruptStackFrame) {
    both_println!("disk irq");

    match LOCAL_APIC.lock().as_mut() {
        Some(apic) => unsafe { apic.end_of_interrupt() },
        None => unsafe { PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8()); }
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(_frame: InterruptStackFrame) {
    crate::time::pit_tick();
    // might not be initialized yet
    if let Some(exc) = crate::task::executor::GLOBAL_EXECUTOR.get() {
        exc.sleep_tick_set();
    }

    match LOCAL_APIC.lock().as_mut() {
        Some(apic) => unsafe { apic.end_of_interrupt() },
        None => unsafe { PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8()); }
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_frame: InterruptStackFrame) {
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::add_scancode(scancode);

    match LOCAL_APIC.lock().as_mut() {
        Some(apic) => unsafe { apic.end_of_interrupt() },
        None => unsafe { PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8()); }
    }
}

extern "x86-interrupt" fn cascade_interrupt_handler(_frame: InterruptStackFrame) {
    both_println!("Cascade interrupt");
    halt_loop();
}

extern "x86-interrupt" fn com1_interrupt_handler(_frame: InterruptStackFrame) {
    both_println!("COM1 interrupt");
    halt_loop();
}

extern "x86-interrupt" fn com2_interrupt_handler(_frame: InterruptStackFrame) {
    both_println!("COM2 interrupt");
    halt_loop();
}

extern "x86-interrupt" fn lpt1_interrupt_handler(_frame: InterruptStackFrame) {
    both_println!("LPT1 interrupt");
    halt_loop();
}

extern "x86-interrupt" fn lpt2_interrupt_handler(_frame: InterruptStackFrame) {
    both_println!("LPT2 interrupt");
    halt_loop();
}

extern "x86-interrupt" fn floppy_interrupt_handler(_frame: InterruptStackFrame) {
    both_println!("Floppy disk interrupt");
    halt_loop();
}

extern "x86-interrupt" fn coprocessor_interrupt_handler(_frame: InterruptStackFrame) {
    both_println!("Coprocessor interrupt");
    halt_loop();
}

extern "x86-interrupt" fn cmos_interrupt_handler(_frame: InterruptStackFrame) {
    both_println!("CMOS interrupt");
    halt_loop();
}

extern "x86-interrupt" fn ps2_mouse_interrupt_handler(_frame: InterruptStackFrame) {
    both_println!("PS2 mouse interrupt");
    halt_loop();
}

// Tests ///////////////////////////////////////////////////////////////////////

#[test_case]
fn test_breakpoint_exception() {
    crate::arch::gdt::init();
    crate::arch::interrupts::early_init_interrupts();
    x86_64::instructions::interrupts::int3();
}
