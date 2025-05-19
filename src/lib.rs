#![no_std]
#![feature(offset_of)]
#![feature(custom_test_frameworks)]
#![feature(lang_items)]
#![test_runner(test_runner::test_runner)]
#![reexport_test_harness_main = "run_unit_tests"]
#![no_main]

pub mod allocator;
pub mod graphics;
pub mod qemu;
pub mod result;
pub mod uefi;
pub mod x86;

#[cfg(test)]
pub mod test_runner;

#[cfg(test)]
#[lang = "eh_personality"]
#[no_mangle]
pub extern "C" fn eh_personality() {}

#[cfg(test)]
#[no_mangle]
pub fn efi_main() {
    run_unit_tests()
}