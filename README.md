# [Untitled OS Project]

[![build-x86_64](https://github.com/trashbyte/os/actions/workflows/x86_64.yaml/badge.svg?branch=master)](https://github.com/trashbyte/os/actions/workflows/x86_64.yaml)

This project is the beginnings of a truly *modern* operating system. What I mean by that is, existing operating systems continue to use outdated systems and hacks for compatibility purposes. This project aims to create a new OS from scratch, which uses the best options available for each aspect of the system.

For example:

 - **Written in pure Rust:** C and C++ carry a lot of historical baggage, and x86 assembly was designed to be written by compilers, not humans. Rust is a much more modern language with much nicer syntax, better type safety, and an extremely powerful compile-time borrow checker that ensures memory safety at compile time, with no runtime overhead for maximum performance.
 - **A modern shell:** Existing shells such as bash or the Windows command prompt are powerful tools, but they suffer from questionable design choices. Bare word commands make for lots of problems with spaces and lots of reserved characters. Using a more traditional scripting syntax would allow for much less error-prone shell scripting, with fewer hacks and workarounds.
 - **Better IPC:** Traditional process interfaces (stdin, stdout, stderr) are limited in functionality and hard to work with due to their unstructured plain-text format. Most OSes offer alternative IPC mechanisms such as sockets and Microsoft's COM, but these add-ons can be clunky and hard to work with, which is a huge negative for an inter-process communication system. A better kernel-level process interface system would make for much easier and more powerful program chaining, allowing much more modularity and reusability in programs, aligning nicely with UNIX's concept of "do one thing well".
 - **Permissions-based security:** Permissions-based security has been implemented in mobile OSes for many years now, and with good reason. There's no reason to give every program on a computer access to nearly every system available, with a single "root" or "administrator" privilege level that exposes *everything* being the only other option. Permissions-based security offers stronger security through a less permissive default policy and finer control over what each program has access to. It also makes the user aware of every system a program accesses, which makes suspicious or undesireable behavior more obvious.
 - **A more convenient filesystem:** As mentioned before, modern shells use bare words for commands, with space acting as a separator and a number of reserved characters like `<`, `>`, `?`, and `|`. Not only are these characters unusable in filenames anywhere on the system solely for the sake of using bare-word syntax in the shell, but something even as benign as including a space in a filename is a common source of problems in existing operating systems. Using different syntax which requires strings to be `"surrounded by quotes"` solves both of these problems quite nicely, and allows for almost every character to be used in a filename. In addition to more permissive file naming and full Unicode support, other planned features include executable directories and per-directory default program overrides. This means you can designate folders to be opened with certain programs, so for example you could open a code project folder in the IDE of your choice automatically. Per-directory default program overrides allow file types to be associated with certain programs in particular places, e.g. `.png` files in your photos folder would open with a slideshow viewer, but `.png` files in a design project folder would open in an image editor. Letting programs register their own custom file metadata types would be nice too.

## Roadmap

As this project is still in its infancy, very few things are currently implemented. Here's an overview of what's done and what's to be done next:

#### Done

 - [X] Simple to build: `cargo build`, `run` and `test` work correctly without any extra arguments
 - [X] QEMU runner (including tests)
 - [X] Virtual memory, address translation
 - [X] PIC and APIC interrupt handling
 - [X] Text-mode VGA output
 - [X] Serial output
 - [X] Heap memory allocator to enable Rust's dynamically-allocated types
 - [X] PCI device discovery
 - [X] Keyboard input
 - [X] Terminal colors, scrollback
 - [X] AHCI driver, support for SATA/SATAPI drives
 - [X] Basic shell (refactoring)
 - [X] RTC
 - [X] Async task executor with cooperative multitasking
 - [X] Support for Rust's `async`/`await` syntax

#### In Progress

 - [ ] Generic block device API, virtual filesystem
 - [ ] Ext2 filesystem support
 - [ ] Basic round-robin preemptive task scheduling

#### Todo

 - [ ] IDE storage driver (reverted)
 - [ ] Process fork/join
 - [ ] IPC
 - [ ] More advanced shell
 - [ ] Program loading
 - [ ] User space
 - [ ] Syscalls
 - [ ] Preemtive scheduler with variable priority
 - [ ] Thread API, thread-local storage
 - [ ] USB
 - [ ] Permissions framework
 - [ ] VESA graphics drivers
 - [ ] Windowing system / desktop environment
 - [ ] Mouse support
 - [ ] Module loader
 - [ ] Hardware multithreading (Symmetric Multiprocessing)
 - [ ] Integrated debugger
 - [ ] Custom filesystem
 - [ ] GUI compositing and such
 - [ ] Networking
 - [ ] Sound
 - [ ] Native Rust stdlib, custom OS cross-compiler

#### Things which I plan to support at some point, but aren't needed to move forward:

 - [ ] UEFI? The bootloader is legacy-BIOS only right now, but I'm not sure I *need* UEFI. I might look into it at some point though.
 - [ ] Floppy disk driver, other old hardware stuff
 - [ ] Support for more filesystems: FAT32, Ext3/4, NTFS
 - [ ] initramfs
 - [ ] various user-space programs

## Supported Platforms

Right now, the kernel targets x86_64, and runs in QEMU. I'd like to get a setup for hardware testing with reasonable iteration time, like maybe network boot or something.

Right now I only have plans to support x86_64, but I may add support for ARM (aarch64) or other architectures at some point in the future.

## How to Build

Uhhh I don't have proper instructions for building although *in theory* it should be fairly simple (thank you cargo). If you want to be a guinea pig, then here's what I'd suggest:

 - Make sure your toolchain is up to date. Set the rustup override (or the default) to nightly. There's plenty of reasons this won't build on stable.
 - Install bootimage (`cargo install bootimage`).
 - Install QEMU and add the executable dir to your path.
 - Run `cargo run` and pray.

If you end up trying this and run into issues, you can email me about it (`github@trashbyte.io`) or ping me on Mastodon ([@trashbyte](https://cybre.space/@trashbyte)) but I make no promises about being able to help you at this time. Also, you can check out [the tutorials here](https://os.phil-opp.com/) since I based my initial setup off of them. It might cover some steps I forgot.

## How to Contribute

Don't.

Okay, I know that's not very Open Source of me, but the project is currently very early in development, and if you asked me what I needed help with I'd have a hard time even telling you what to work on.

I definitely want to accept contributions at some point, since that's a big part of what makes open source development great. But at least for the time being, just let me hammer this into something vaguely OS-shaped. Check back in a month or so.

And if you're *really* curious, of course you can always look at the source, although it's a mess and poorly documented (again, I'm working on it).
