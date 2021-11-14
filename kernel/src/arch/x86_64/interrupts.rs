// The MIT License (MIT)
// Copyright (c) 2020 trashbyte
// See LICENSE.txt for full license

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use spin;
use lazy_static::lazy_static;
use crate::{print, println};
use crate::util::halt_loop;


lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
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
        idt[InterruptIndex::Cascade.as_usize()].set_handler_fn(generic_interrupt_handler);
        idt[InterruptIndex::Com2.as_usize()].set_handler_fn(generic_interrupt_handler);
        idt[InterruptIndex::Com1.as_usize()].set_handler_fn(generic_interrupt_handler);
        idt[InterruptIndex::LPT2.as_usize()].set_handler_fn(generic_interrupt_handler);
        idt[InterruptIndex::FloppyDisk.as_usize()].set_handler_fn(generic_interrupt_handler);
        idt[InterruptIndex::LPT1.as_usize()].set_handler_fn(generic_interrupt_handler);
        idt[InterruptIndex::CMOS.as_usize()].set_handler_fn(generic_interrupt_handler);
        idt[InterruptIndex::Peripheral1.as_usize()].set_handler_fn(disk_irq_handler);
        idt[InterruptIndex::Peripheral2.as_usize()].set_handler_fn(disk_irq_handler);
        idt[InterruptIndex::Peripheral3.as_usize()].set_handler_fn(disk_irq_handler);
        idt[InterruptIndex::PS2Mouse.as_usize()].set_handler_fn(generic_interrupt_handler);
        idt[InterruptIndex::Coprocessor.as_usize()].set_handler_fn(generic_interrupt_handler);
        idt[InterruptIndex::PrimaryATA.as_usize()].set_handler_fn(disk_irq_handler);
        idt[InterruptIndex::SecondaryATA.as_usize()].set_handler_fn(disk_irq_handler);
        idt
    };
}

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
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: &mut InterruptStackFrame, _e: u64) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(frame: &mut InterruptStackFrame, err: PageFaultErrorCode) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", err);
    println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn non_maskable_interrupt_handler(frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: NMI");
    println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn alignment_check_handler(frame: &mut InterruptStackFrame, error_code: u64) {
    println!("EXCEPTION: ALIGNMENT CHECK");
    println!("Error code: {:X}", error_code);
    println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn bound_range_exceeded_handler(frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: BOUND RANGE EXCEEDED");
    println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn segment_not_present_handler(frame: &mut InterruptStackFrame, error_code: u64) {
    println!("EXCEPTION: SEGMENT NOT PRESENT");
    println!("Error code: {:X}", error_code);
    println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn general_protection_fault_handler(frame: &mut InterruptStackFrame, error_code: u64) {
    println!("EXCEPTION: GENERAL PROTECTION FAULT");
    println!("Error code: {:X}", error_code);
    println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn device_not_available_handler(frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: DEVICE NOT PRESENT");
    println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn divide_error_handler(frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: DIVIDE ERROR");
    println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn security_exception_handler(frame: &mut InterruptStackFrame, error_code: u64) {
    println!("EXCEPTION: SECURITY EXCEPTION");
    println!("Error code: {:X}", error_code);
    println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn simd_floating_point_handler(frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: SIMD FLOATING POINT ERROR");
    println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn x87_floating_point_handler(frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: X87 FLOATING POINT ERROR");
    println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn stack_segment_fault_handler(frame: &mut InterruptStackFrame, error_code: u64) {
    println!("EXCEPTION: STACK SEGMENT FAULT HANDLER");
    println!("Error code: {:X}", error_code);
    println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn invalid_tss_handler(frame: &mut InterruptStackFrame, error_code: u64) {
    println!("EXCEPTION: INVALID TSS");
    println!("Error code: {:X}", error_code);
    println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn invalid_opcode_handler(frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: INVALID OPCODE");
    println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn machine_check_handler(frame: &mut InterruptStackFrame) -> ! {
    println!("EXCEPTION: MACHINE CHECK");
    println!("{:#?}", frame);
    halt_loop()
}

extern "x86-interrupt" fn overflow_handler(frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: OVERFLOW");
    println!("{:#?}", frame);
    halt_loop();
}

extern "x86-interrupt" fn disk_irq_handler(_frame: &mut InterruptStackFrame) {
    println!("disk irq");
    unsafe { Port::<u8>::new(0x20).write(0x20); }
}

static mut TICKS: u64 = 0;
pub fn ticks() -> u64 { unsafe { TICKS } }

extern "x86-interrupt" fn timer_interrupt_handler(_frame: &mut InterruptStackFrame) {
    unsafe {
        TICKS += 5;
    }
//    if (*lock) % 1000 == 0 {
//        print!(".");
//    }
    unsafe { Port::<u8>::new(0x20).write(0x20); }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_frame: &mut InterruptStackFrame) {
    use pc_keyboard::{Keyboard, ScancodeSet1, DecodedKey, layouts};
    use spin::Mutex;

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1));
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event.clone()) {
            drop(keyboard);
            match key {
                DecodedKey::Unicode(character) => {
                    if character == '\x08' {
                        // true if a character was erased (so we can't go past the start of the string)
                        if (*crate::shell::SHELL.lock()).backspace() {
                            crate::vga_buffer::_backspace();
                        }
                    }
                    else {
                        print!("{}", character);
                        if character == '\n' {
                            (*crate::shell::SHELL.lock()).submit();
                        }
                        else {
                            (*crate::shell::SHELL.lock()).add_char(character);
                        }
                    }
                },
                DecodedKey::RawKey(key) => {
                    if key_event.state == KeyState::Down {
                        match key {
                            KeyCode::PageUp => (*crate::vga_buffer::TERMINAL.lock()).scroll(true),
                            KeyCode::PageDown => (*crate::vga_buffer::TERMINAL.lock()).scroll(false),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    unsafe { Port::<u8>::new(0x20).write(0x20); }
}

extern "x86-interrupt" fn generic_interrupt_handler(_frame: &mut InterruptStackFrame) {
    println!("Unknown generic interrupt");
    halt_loop();
}

// Tests ///////////////////////////////////////////////////////////////////////


#[cfg(test)]
use crate::{serial_print, serial_println};
use x86_64::instructions::port::Port;
use pc_keyboard::{KeyCode, KeyState};

#[test_case]
fn test_breakpoint_exception() {
    serial_print!("test_breakpoint_exception...");
    x86_64::instructions::interrupts::int3();
    serial_println!("[ok]");
}
