#![feature(offset_of)]
#![no_std]
#![no_main]

use core::arch::asm;
use core::fmt::Write;
use core::panic::PanicInfo;
use core::writeln;
use testOS::graphics::draw_test_pattern;
use testOS::graphics::fill_rect;
use testOS::graphics::Bitmap;
use testOS::uefi::init_vram;
use  testOS::uefi::EfiHandle;
use testOS::uefi::EfiMemoryType;
use testOS::uefi::EfiSystemTable;
use testOS::uefi::exit_from_efi_boot_services;
use testOS::uefi::MemoryMapHolder;
use testOS::uefi::VramTextWriter;
use testOS::qemu::exit_qemu;
use testOS::qemu::QemuExitCode;
use testOS::x86::hlt;

// EFIのエントリポイント
#[no_mangle]
fn efi_main(
    image_handle: EfiHandle,
    efi_system_table: &EfiSystemTable,
) -> ! {

    let mut vram = init_vram(efi_system_table).expect("Failed to initialize VRAM");
    let vw = vram.width();
    let vh = vram.height();
    fill_rect(&mut vram, 0, 0, vw, vh, 0x000000).expect("Failed to fill rect");
    draw_test_pattern(&mut vram);
    let mut w = VramTextWriter::new(&mut vram);
    for i in 0..4 {
        writeln!(w, "i = {i}").unwrap();
    }

    let mut memory_map = MemoryMapHolder::new();
    let status = efi_system_table.boot_services().get_memory_map(&mut memory_map);
    writeln!(w, "EFI_STATUS: {status:?}").unwrap();
    let mut total_memory_page = 0;
    for e in memory_map.iter() {
        if e.memory_type() != EfiMemoryType::CONVENTIONAL_MEMORY {
            continue;
        }
        total_memory_page += e.number_of_pages();
        writeln!(w, "{e:?}").unwrap();
    }
    let total_memory_size = total_memory_page * 4096 / 1024 / 1024;
    writeln!(w, "Total memory size: {total_memory_size}MiB").unwrap();

    //println!("Hello, world!");
    exit_from_efi_boot_services(image_handle, efi_system_table, &mut memory_map);
    writeln!(w, "Exit from EFI boot services").unwrap();
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    exit_qemu(QemuExitCode::Failure);
}
