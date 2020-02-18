use core::fmt;
use volatile::Volatile;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::interrupts;

const BLANK_CHAR: ScreenChar = ScreenChar {
    ascii_character: ' ' as u8,
    color_code: ColorCode(0)
};

lazy_static! {
    pub static ref TERMINAL: Mutex<Terminal> = Mutex::new(Terminal {
        cursor_col: 0,
        cursor_row: 0,
        scroll_row: 0,
        color_code: ColorCode::new(Color::White, Color::Black),
        scrollback: [[BLANK_CHAR; SCREEN_WIDTH]; 1000],
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

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;

    interrupts::without_interrupts(|| {
        TERMINAL.lock().write_fmt(args).unwrap();
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
struct Buffer {
    chars: [[Volatile<ScreenChar>; SCREEN_WIDTH]; SCREEN_HEIGHT]
}


pub struct Terminal {
    cursor_col: usize,
    cursor_row: usize,
    scroll_row: usize,
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
                self.scrollback[self.cursor_row][self.cursor_col] = c;
                self.cursor_col += 1;
                if self.cursor_col >= SCREEN_WIDTH {
                    self.new_line();
                }
                else {
                    // only draw new char unless we need to scroll
                    self.screen_buffer.chars[self.cursor_row.min(SCREEN_HEIGHT-1)][self.cursor_col-1].write(c);
                }
            }
        }
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
        //self.set_cursor_pos(self.cursor_col as u8, self.cursor_row as u8 - self.scroll_row as u8);
    }

    fn refresh(&mut self) {
        let first_row_to_draw = self.scroll_row;
        let last_row_to_draw = (self.scroll_row + SCREEN_HEIGHT).min(self.cursor_row);
        let mut write_row = 0;
        for y in first_row_to_draw..(last_row_to_draw + 1) {
            for x in 0..SCREEN_WIDTH {
                let character = self.scrollback[y][x];
                self.screen_buffer.chars[write_row][x].write(character);
            }
            write_row += 1;
        }
    }

    #[allow(dead_code)]
    fn cursor_pos(&self) -> (u8, u8) {
        unsafe {
            let mut port1 = Port::<u8>::new(0x3D4);
            let mut port2 = Port::<u8>::new(0x3D5);
            port1.write(0x0F);
            let x = port2.read();
            port1.write(0x0E);
            let y = port2.read();
            (x, y)
        }
    }

    #[allow(dead_code)]
    fn set_cursor_pos(&self, x: u8, y: u8) {
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
        if self.cursor_col > 0 {
            self.cursor_col -= 1;

            self.scrollback[self.cursor_row][self.cursor_col] = BLANK_CHAR;
            self.screen_buffer.chars[self.cursor_row.min(SCREEN_HEIGHT-1)][self.cursor_col].write(BLANK_CHAR);
        }
        else if self.cursor_row > 0 { // wrap
            self.cursor_col = SCREEN_WIDTH-1;
            self.cursor_row -= 1;

            self.scrollback[self.cursor_row][self.cursor_col] = BLANK_CHAR;
            self.screen_buffer.chars[self.cursor_row.min(SCREEN_HEIGHT-1)][self.cursor_col].write(BLANK_CHAR);
        }
        // else, we're trying to wrap around at the first line, do nothing
    }

//    fn scroll(&mut self, add: bool) {
//        if add {
//            if self.lines_written == 0 { return; } // can't scroll anywhere on first line
//            else { self.scroll_row += 1; }
//        }
//        else {
//            if self.scroll_row == 0 { return; } // cant scroll past the first line
//        }
//
//        if add { self.scroll_row += 1; }
//        else { self.scroll_row -= 1; }
//    }

    fn new_line(&mut self) {
        self.cursor_row += 1;
        if self.cursor_row >= 1000 { panic!("oops i havent handled scrollback overflow yet"); }
        self.cursor_col = 0;

        if self.cursor_row - self.scroll_row >= SCREEN_HEIGHT {
            self.scroll_row += 1;
        }

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
        let mut writer = WRITER.lock();
        writeln!(writer, "\n{}", s).expect("writeln failed");
        for (i, c) in s.chars().enumerate() {
            let screen_char = writer.buffer.chars[SCREEN_HEIGHT - 2][i].read();
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
        let screen_char = WRITER.lock().buffer.chars[SCREEN_HEIGHT - 2][i].read();
        assert_eq!(char::from(screen_char.ascii_character), c);
    }

    serial_println!("[ok]");
}

#[test_case]
fn test_wrapping() {
    serial_print!("test_wrapping... ");

    let s = "A different, much longer string, one that is so long that it exceeds the length of the buffer and must be wrapped";
    print!("{}", s);
    for (i, c) in s.chars().enumerate() {
        let row = if i < 80 { SCREEN_HEIGHT - 2 } else { SCREEN_HEIGHT - 1 };
        let col = if i < 80 { i } else { i - 80 };
        let screen_char = WRITER.lock().buffer.chars[row][col].read();
        assert_eq!(char::from(screen_char.ascii_character), c);
    }

    serial_println!("[ok]");
}
