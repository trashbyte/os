//! CPU-level input/output instructions, including `inb`, `outb`, etc., and
//! a high level Rust wrapper.

use core::marker::PhantomData;

// Rust wrappers around the x86-family I/O instructions.

/// Read a `u8`-sized value from `port`.
pub unsafe fn inb(port: u16) -> u8 {
    // The registers for the `in` and `out` instructions are always the
    // same: `a` for value, and `d` for the port address.
    let result: u8;
    asm!("in al, dx", out("al") result, in("dx") port);
    result
}

/// Write a `u8`-sized `value` to `port`.
pub unsafe fn outb(value: u8, port: u16) {
    asm!("out dx, al", in("dx") port, in("al") value);
}

/// Read a `u16`-sized value from `port`.
pub unsafe fn inw(port: u16) -> u16 {
    let result: u16;
    asm!("in ax, dx", out("ax") result, in("dx") port);
    result
}

/// Write a `u8`-sized `value` to `port`.
pub unsafe fn outw(value: u16, port: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") value);
}

/// Read a `u32`-sized value from `port`.
pub unsafe fn inl(port: u16) -> u32 {
    let result: u32;
    asm!("in eax, dx", out("eax") result, in("dx") port);
    result
}

/// Write a `u32`-sized `value` to `port`.
pub unsafe fn outl(value: u32, port: u16) {
    asm!("out dx, eax", in("dx") port, in("eax") value);
}

/// This trait is defined for any type which can be read or written over a
/// port.  The processor supports I/O with `u8`, `u16` and `u32`.  The
/// functions in this trait are all unsafe because they can write to
/// arbitrary ports.
pub trait InOut {
    /// Read a value from the specified port.
    unsafe fn port_in(port: u16) -> Self;

    /// Write a value to the specified port.
    unsafe fn port_out(port: u16, value: Self);
}

impl InOut for u8 {
    unsafe fn port_in(port: u16) -> u8 { inb(port) }
    unsafe fn port_out(port: u16, value: u8) { outb(value, port); }
}

impl InOut for u16 {
    unsafe fn port_in(port: u16) -> u16 { inw(port) }
    unsafe fn port_out(port: u16, value: u16) { outw(value, port); }
}

impl InOut for u32 {
    unsafe fn port_in(port: u16) -> u32 { inl(port) }
    unsafe fn port_out(port: u16, value: u32) { outl(value, port); }
}

/// An I/O port over an arbitrary type supporting the `InOut` interface.
///
/// This version of `Port` has safe `read` and `write` functions, and it's
/// appropriate for communicating with hardware that can't violate Rust's
/// safety guarantees.
#[derive(Debug)]
pub struct Port<T: InOut> {
    // Port address.
    port: u16,

    // Zero-byte placeholder.  This is only here so that we can have a
    // type parameter `T` without a compiler error.
    phantom: PhantomData<T>,
}

impl<T: InOut> Port<T> {
    /// Create a new I/O port.
    pub const unsafe fn new(port: u16) -> Port<T> {
        Port { port: port, phantom: PhantomData }
    }

    /// Read data from the port.  This is nominally safe, because you
    /// shouldn't be able to get hold of a port object unless somebody
    /// thinks it's safe to give you one.
    pub fn read(&mut self) -> T {
        unsafe { T::port_in(self.port) }
    }

    /// Write data to the port.
    pub fn write(&mut self, value: T) {
        unsafe { T::port_out(self.port, value); }
    }
}

/// An unsafe I/O port over an arbitrary type supporting the `InOut`
/// interface.
///
/// This version of `Port` has unsafe `read` and `write` functions, and
/// it's appropriate for speaking to hardware that can potentially corrupt
/// memory or cause undefined behavior.
#[derive(Debug)]
pub struct UnsafePort<T: InOut> {
    port: u16,
    phantom: PhantomData<T>,
}

impl<T: InOut> UnsafePort<T> {
    /// Create a new I/O port.
    pub const unsafe fn new(port: u16) -> UnsafePort<T> {
        UnsafePort { port: port, phantom: PhantomData }
    }

    /// Read data from the port.
    pub unsafe fn read(&mut self) -> T {
        T::port_in(self.port)
    }

    /// Write data to the port.
    pub unsafe fn write(&mut self, value: T) {
        T::port_out(self.port, value);
    }
}
