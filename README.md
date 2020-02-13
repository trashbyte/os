# [Untitled OS Project]

This project is the beginnings of a truly *modern* operating system. What I mean by that is, existing operating systems continue to use outdated systems and hacks for compatibility purposes. This project aims to create a new OS from scratch, which uses the best options available for each aspect of the system.

For example:

 - **Written in pure Rust:** C and C++ carry a lot of historical baggage, and x86 assembly was designed to be written by compilers, not humans. Rust is a much more modern language with much nicer syntax, better type safety, and an extremely powerful compile-time borrow checker that ensures memory safety at compile time, with no runtime overhead for maximum performance.
 - **A modern shell:** Existing shells such as bash or the Windows command prompt are powerful tools, but they suffer from questionable design choices. Bare word commands make for lots of problems with spaces and lots of reserved characters. Using a more traditional scripting syntax would allow for much less error-prone shell scripting, with fewer hacks and workarounds.
 - **Better IPC:** Traditional process interfaces (stdin, stdout, stderr) are limited in functionality and hard to work with due to their unstructured plain-text format. Most OSes offer alternative IPC mechanisms such as sockets and Microsoft's COM, but these add-ons can be clunky and hard to work with, which is a huge negative for an inter-process communication system. A better kernel-level process interface system would make for much easier and more powerful program chaining, allowing much more modularity and reusability in programs, aligning nicely with UNIX's concept of "do one thing well".
 - **Permissions-based security:** Permissions-based security has been implemented in mobile OSes for many years now, and with good reason. There's no reason to give every program on a computer access to nearly every system available, with a single "root" or "administrator" privilege level that exposes *everything* being the only other option. Permissions-based security offers stronger security through a less permissive default policy and finer control over what each program has access to. It also makes the user aware of every system a program accesses, which makes suspicious or undesireable behavior more obvious.
 - **A more convenient filesystem:** As mentioned before, modern shells use bare words for commands, with space acting as a separator and a number of reserved characters like `<`, `>`, `?`, and `|`. Not only are these characters unusable in filenames anywhere on the system solely for the sake of using bare-word syntax in the shell, but something even as benign as including a space in a filename is a common source of problems in existing operating systems. Using different syntax which requires strings to be `"surrounded by quotes"` solves both of these problems quite nicely, and allows for almost every character to be used in a filename. In addition to more permissive file naming and full Unicode support, other planned features include executable directories and per-directory default program overrides. This means you can designate folders to be opened with certain programs, so for example you could open a code project folder in the IDE of your choice automatically. Per-directory default program overrides allow file types to be associated with certain programs in particular places, e.g. `.png` files in your photos folder would open with a slideshow viewer, but `.png` files in a design project folder would open in an image editor.

## Roadmap

As this project is still in its infancy, very few things are currently implemented. Here's an overview of what's done and what's to be done next:

 - [X] `cargo` cross-compilation toolchain (including full `cargo test` support and fast iteration with QEMU)
 - [X] Text-mode VGA output
 - [X] Serial output
 - [X] Interrupt handling
 - [X] Virtual memory and rust global allocator for collection types
 - [X] PCI device discovery
 - [X] Keyboard input
 - [X] ATA/IDE storage driver
 - [ ] Ext2 filesystem support (in progress)
 - [ ] Terminal colors, scrollback
 - [ ] Timers
 - [ ] Shell
 - [ ] Basic round-robin task scheduling
 - [ ] Preemtive scheduler with variable priority
 - [ ] Process fork/join
 - [ ] IPC
 - [ ] Module loader
 - [ ] Generic block device interface
 - [ ] Hardware multithreading (Symmetric Multiprocessing)
 - [ ] Internal debugger
 - [ ] User space
 - [ ] USB
 - [ ] Program loading
 - [ ] Syscalls
 - [ ] Permissions framework
 - [ ] Custom filesystem
 - [ ] Thread API, thread-local storage
 - [ ] VESA graphics drivers
 - [ ] Windowing system / desktop environment
 - [ ] GUI compositing
 - [ ] Mouse support
 - [ ] Networking
 - [ ] Sound
 - [ ] Rust `async`/`await` support
 - [ ] Native Rust stdlib, custom OS cross-compiler

Things which I plan to support at some point, but aren't needed to move forward:
 - [ ] UEFI?
 - [ ] AHCI/SATA driver (actually largely written already but I can't seem to get it to work)
 - [ ] Floppy disk driver
 - [ ] Support for more filesystems: FAT32, Ext3/4, NTFS
 - [ ] initramfs
 - [ ] various user-space programs

Currently I only have plans to support x86_64, but I may consider adding support for ARM at some point in the future.