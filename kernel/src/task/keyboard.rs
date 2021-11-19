///////////////////////////////////////////////////////////////////////////////L
// The MIT License (MIT)
// Copyright (c) 2021 [untitled os] Team
// See LICENSE.txt and CREDITS.txt for details
///////////////////////////////////////////////////////////////////////////////L

use conquer_once::spin::OnceCell;
use crossbeam::queue::ArrayQueue;
use pc_keyboard::{ScancodeSet1, layouts, Keyboard, HandleControl, DecodedKey, KeyState, KeyCode};
use futures_util::{Stream, StreamExt};
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_util::task::AtomicWaker;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

/// Called by the keyboard interrupt handler
/// Must not block or allocate.
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if queue.push(scancode).is_err() {
            crate::both_println!("WARNING: scancode queue full; dropping keyboard input");
        }
        else {
            WAKER.wake();
        }
    } else {
        crate::both_println!("WARNING: scancode queue uninitialized");
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScancodeStream {
    // ensure a ScancodeStream can only be created through new()
    _private: (),
}

impl ScancodeStream {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        SCANCODE_QUEUE.try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue not initialized");

        // fast path
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(cx.waker());
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

pub async fn process_scancodes() {
    let mut stream = ScancodeStream::new();
    let mut keyboard = Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::MapLettersToUnicode);

    while let Some(scancode) = stream.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event.clone()) {
                match key {
                    DecodedKey::Unicode(character) => {
                        if character == '\x08' {
                            // true if a character was erased (so we can't go past the start of the string)
                            if (*crate::shell::SHELL.lock()).backspace() {
                                crate::vga_buffer::_backspace();
                            }
                        }
                        else {
                            crate::print!("{}", character);
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
    }
}
