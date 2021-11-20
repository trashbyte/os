///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use core::fmt;
use volatile::Volatile;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::interrupts;

const BLANK_CHAR: ScreenChar = ScreenChar {
    ascii_character: b' ',
    color_code: ColorCode(0)
};

const SCROLLBACK_LINES: usize = 1000;

lazy_static! {
    pub static ref TERMINAL: Mutex<Terminal> = Mutex::new(Terminal {
        cursor_x: 0,
        cursor_y: 0,
        scroll_row: 0,
        color_code: ColorCode::new(Color::White, Color::Black),
        scrollback: [[BLANK_CHAR; SCREEN_WIDTH]; SCROLLBACK_LINES],
        screen_buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}


#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! both_print {
    ($($arg:tt)*) => {
        $crate::vga_buffer::_print_both(format_args!($($arg)*))
    }
}

#[macro_export]
macro_rules! both_println {
    () => ($crate::both_print!("\n"));
    ($($arg:tt)*) => ($crate::both_print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments<'_>) {
    use core::fmt::Write;

    interrupts::without_interrupts(|| {
        TERMINAL.lock().write_fmt(args)
            .expect("Printing to terminal failed");
    });
}

#[doc(hidden)]
pub fn _print_both(args: fmt::Arguments<'_>) {
    use core::fmt::Write;

    interrupts::without_interrupts(|| {
        crate::device::serial::SERIAL1
            .lock()
            .write_fmt(args)
            .expect("Printing to serial failed");
    });
    interrupts::without_interrupts(|| {
        TERMINAL.lock().write_fmt(args)
            .expect("Printing to terminal failed");
    });
}

#[doc(hidden)]
pub fn _backspace() {
    interrupts::without_interrupts(|| {
        TERMINAL.lock().backspace();
    });
}


#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const SCREEN_HEIGHT: usize = 25;
const SCREEN_WIDTH: usize = 80;

#[repr(transparent)]
#[derive(Debug)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; SCREEN_WIDTH]; SCREEN_HEIGHT]
}

#[derive(Debug)]
pub struct Terminal {
    /// Cursor column in screen space (0..SCREEN_WIDTH)
    cursor_x: usize,
    /// Cursor row in screen space (0..SCREEN_HEIGHT)
    cursor_y: usize,
    /// Index of scrollback entry for the first line of the screen
    scroll_row: usize,
    /// Current color code for new chars
    color_code: ColorCode,
    // TODO: use ring buffer, handle overflow properly
    scrollback: [[ScreenChar; SCREEN_WIDTH]; 1000],
    screen_buffer: &'static mut Buffer,
}

impl Terminal {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.new_line();
            },
            byte => {
                let c = ScreenChar {
                    ascii_character: byte,
                    color_code: self.color_code,
                };
                self.set_char_at_cursor(c);

                if self.cursor_x == SCREEN_WIDTH-1 {
                    self.new_line();
                }
                else {
                    self.set_cursor_x(self.cursor_x + 1);
                }
            }
        }
        self.scroll_to_bottom();
    }

    fn set_char_at_cursor(&mut self, c: ScreenChar) {
        self.scrollback[self.cursor_y + self.scroll_row][self.cursor_x] = c;
        self.screen_buffer.chars[self.cursor_y][self.cursor_x].write(c);
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn refresh(&mut self) {
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let character = self.scrollback[y + self.scroll_row][x];
                self.screen_buffer.chars[y][x].write(character);
            }
        }
    }

    pub fn set_color(&mut self, fg: Color, bg: Color) {
        self.color_code = ColorCode::new(fg, bg);
    }

    pub fn cursor_x(&self) -> usize {
        self.cursor_x
    }

    pub fn set_cursor_x(&mut self, value: usize) {
        self.cursor_x = value % SCREEN_WIDTH;
        self.set_vga_cursor_pos(self.cursor_x as u8, self.cursor_y as u8);
    }

    pub fn cursor_y(&self) -> usize {
        self.cursor_y
    }

    pub fn set_cursor_y(&mut self, value: usize) {
        self.cursor_y = value % SCREEN_HEIGHT;
        self.set_vga_cursor_pos(self.cursor_x as u8, self.cursor_y as u8);
    }

    fn set_vga_cursor_pos(&self, x: u8, y: u8) {
        let pos = y as u16 * 80 + x as u16;
        unsafe {
            let mut port1 = Port::<u8>::new(0x3D4);
            let mut port2 = Port::<u8>::new(0x3D5);
            port1.write(0x0F);
            port2.write((pos & 0xFF) as u8);
            port1.write(0x0E);
            port2.write(((pos >> 8) & 0xFF) as u8);
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor_x > 0 {
            self.set_cursor_x(self.cursor_x - 1);
            self.set_char_at_cursor(BLANK_CHAR);
        }
        else if self.cursor_y > 0 {
            // backspace at first column, wrap to prev line
            self.set_cursor_x(SCREEN_WIDTH - 1);
            self.set_cursor_y(self.cursor_y - 1);
            self.set_char_at_cursor(BLANK_CHAR);
        }
        // else, we're trying to wrap around at the first line, do nothing
    }

    // TODO: scrolling up and then typing input breaks scrollback
    pub fn scroll(&mut self, up: bool) {
        if up {
            if self.scroll_row > 0 {
                self.scroll_row -= 1;
            }
        }
        else if self.scroll_row < SCROLLBACK_LINES-1 {
            self.scroll_row += 1;
        }
        self.refresh();
    }

    pub fn scroll_to_bottom(&mut self) {
        if self.cursor_y + self.scroll_row >= SCREEN_HEIGHT {
            self.scroll_row = (self.cursor_y + self.scroll_row) - SCREEN_HEIGHT + 1;
            self.refresh();
        }
    }

    fn new_line(&mut self) {
        if self.cursor_y >= SCREEN_HEIGHT-1 {
            self.set_cursor_y(SCREEN_HEIGHT-1);
            self.scroll_row += 1;
            if self.scroll_row >= 1000 - SCREEN_WIDTH { panic!("oops i havent handled scrollback overflow yet"); }
        }
        else {
            self.set_cursor_y(self.cursor_y + 1);
        }

        self.set_cursor_x(0);
        self.refresh();
    }
}


impl fmt::Write for Terminal {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}


// Tests ///////////////////////////////////////////////////////////////////////

#[cfg(test)]
use crate::{serial_print, serial_println};
use x86_64::instructions::port::Port;

#[test_case]
fn test_println_simple() {
    serial_print!("test_println... ");
    println!("test_println_simple output");
    serial_println!("[ok]");
}

#[test_case]
fn test_println_many() {
    serial_print!("test_println_many... ");
    for _ in 0..200 {
        println!("test_println_many output");
    }
    serial_println!("[ok]");
}

#[test_case]
fn test_println_output() {
    use core::fmt::Write;
    serial_print!("test_println_output... ");

    let s = "Some test string that fits on a single line";
    interrupts::without_interrupts(|| {
        let mut writer = TERMINAL.lock();
        writeln!(writer, "\n{}", s).expect("writeln failed");
        for (i, c) in s.chars().enumerate() {
            let screen_char = writer.screen_buffer.chars[SCREEN_HEIGHT - 2][i].read();
            assert_eq!(char::from(screen_char.ascii_character), c);
        }
    });

    serial_println!("[ok]");
}

#[test_case]
fn test_newline() {
    serial_print!("test_newline... ");

    let s = "test\n";
    print!("{}", s);
    for (i, c) in s.chars().enumerate() {
        if c == '\n' { continue }
        let screen_char = TERMINAL.lock().screen_buffer.chars[SCREEN_HEIGHT - 2][i].read();
        assert_eq!(char::from(screen_char.ascii_character), c);
    }

    serial_println!("[ok]");
}

#[test_case]
fn test_wrapping() {
    serial_print!("test_wrapping... ");

    for _ in 0..SCREEN_HEIGHT+1 {
        println!(); // ensure cursor is at the bottom of the terminal
    }
    let s = "A different, much longer test string (a string used for testing), a string that is soooooo long that it exceeds the length of the buffer and must be wrapped around on to the next line of the display.";
    print!("{}", s);
    for (i, c) in s.chars().enumerate() {
        let row_offset = (s.len() - i + SCREEN_WIDTH/2) / SCREEN_WIDTH; // rounded down
        let row = SCREEN_HEIGHT - 1 - row_offset;
        let col = i % SCREEN_WIDTH;
        let screen_char = TERMINAL.lock().screen_buffer.chars[row][col].read();
        assert_eq!(char::from(screen_char.ascii_character), c);
    }

    serial_println!("[ok]");
}
