#![no_std]
#![no_main]

use core::{cmp::min, mem::size_of, panic::PanicInfo, ptr::null_mut};
type EfiHandle = u64;
type EfiVoid = u8;
type Result<T> = core::result::Result<T, &'static str>;


#[repr(C)]
struct EfiBootServicesTable {
    _reserved: [u64; 40],
    locate_protocol: unsafe extern "win64" fn(
        protocol: *const EfiGuid,
        registration: *const EfiVoid,
        interface: *mut *mut EfiVoid,
    ) -> EfiStatus,
}

#[repr(C)]
struct EfiSystemTable {
    _reserved0: [u64; 12],
    pub boot_services: &'static EfiBootServicesTable,
}

const EFI_GRAPHGICS_OUTPUT_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data0: 0x9042a9de,
    data1: 0x23dc,
    data2: 0x4a38,
    data3: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct EfiGuid {
    data0: u32,
    data1: u16,
    data2: u16,
    data3: [u8; 8],
}

#[repr(C)]
#[derive(Debug)]
struct EfiGraphicsOutputProtocolMode<'a> {
    pub max_mode: u32,
    pub mode: u32,
    pub info: &'a EfiGraphicsOutputProtocolPixelInfo,
    pub size_of_info: u64,
    pub frame_buffer_base: usize,
    pub frame_buffer_size: usize,
}

#[repr(C)]
#[derive(Debug)]
struct EfiGraphicsOutputProtocol<'a> {
    reserved: [u64; 3],
    pub mode: &'a EfiGraphicsOutputProtocolMode<'a>,
}

#[repr(C)]
#[derive(Debug)]
struct EfiGraphicsOutputProtocolPixelInfo {
    version: u32,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    _padding0: [u32; 5],
    pub pixel_per_scan_line: u32,
}
const _: () = assert!(size_of::<EfiGraphicsOutputProtocolPixelInfo>() == 36);

fn locate_graphics_protocol<'a>(
    efi_system_table: &EfiSystemTable,
) -> Result<&'a EfiGraphicsOutputProtocol<'a>> {
    let mut graphic_output_protocol = null_mut::<EfiGraphicsOutputProtocol>();
    unsafe {
        let status = (efi_system_table.boot_services.locate_protocol)(
            &EFI_GRAPHGICS_OUTPUT_PROTOCOL_GUID,
            null_mut::<EfiVoid>(),
            &mut graphic_output_protocol as *mut *mut EfiGraphicsOutputProtocol as *mut *mut EfiVoid,
        );
        if status != EfiStatus::Success {
            return Err("Failed to locate graphics output protocol");
        }
        Ok(&*graphic_output_protocol)
    }
}

#[derive(Debug, PartialEq, Eq,Clone, Copy)]
#[must_use]
#[repr(u64)]
enum EfiStatus {
    Success = 0,
}

trait Bitmap {
    fn bytes_per_pixel(&self) -> i64;
    fn pixels_per_line(&self) -> i64;
    fn width(&self) -> i64;
    fn height(&self) -> i64;
    fn buf_mut(&mut self) -> *mut u8;

    unsafe fn unchecked_pixel_at_mut(
        &mut self,
        x: i64,
        y: i64,
    ) -> *mut u32 {
        unsafe { self.buf_mut().add(
            ((y * self.pixels_per_line() + x) * self.bytes_per_pixel())
            as usize,) as *mut u32 }
    }

    fn pixel_at_mut(
        &mut self,
        x: i64,
        y: i64,
    ) -> Option<&mut u32> {
        if self.is_in_x_range(x) && self.is_in_y_range(y) {
            unsafe { Some(&mut * (self.unchecked_pixel_at_mut(x, y))) }
        } else {
            None
        }
    }

    fn is_in_x_range(&self, px: i64) -> bool {
        0 <= px && px < min(self.width(), self.pixels_per_line())
    }

    fn is_in_y_range(&self, py: i64) -> bool {
        0 <= py && py < self.height()
    }
}

#[derive(Clone, Copy)]
struct VramBufferInfo {
    buf: *mut u8,
    width: i64,
    height: i64,
    pixels_per_line: i64,
}

impl Bitmap for VramBufferInfo {
    fn bytes_per_pixel(&self) -> i64 {
        4
    }

    fn pixels_per_line(&self) -> i64 {
        self.pixels_per_line
    }

    fn width(&self) -> i64 {
        self.width
    }

    fn height(&self) -> i64 {
        self.height
    }

    fn buf_mut(&mut self) -> *mut u8 {
        self.buf
    }
}

fn init_vram(
    efi_system_table: &EfiSystemTable,
) -> Result<VramBufferInfo> {
    let gp = locate_graphics_protocol(efi_system_table)?;
    Ok(VramBufferInfo {
        buf: gp.mode.frame_buffer_base as *mut u8,
        width: gp.mode.info.horizontal_resolution as i64,
        height: gp.mode.info.vertical_resolution as i64,
        pixels_per_line: gp.mode.info.pixel_per_scan_line as i64,
    })
}

unsafe fn unchecked_draw_point<T: Bitmap>(
    buf: &mut T,
    x: i64,
    y: i64,
    color: u32,
) {
    unsafe { *buf.unchecked_pixel_at_mut(x, y) = color };
}

fn draw_point<T: Bitmap>(
    buf: &mut T,
    x: i64,
    y: i64,
    color: u32,
) -> Result<()> {
    *(buf.pixel_at_mut(x, y).ok_or("Out of bounds")?) = color;
    Ok(())
}

fn fill_rect<T: Bitmap>(
    buf: &mut T,
    px: i64,
    py: i64,
    width: i64,
    height: i64,
    color: u32,
) -> Result<()> {
    if !buf.is_in_x_range(px)
        || !buf.is_in_y_range(py)
        || !buf.is_in_x_range(px + width - 1)
        || !buf.is_in_y_range(py + height - 1) {
        return Err("Out of bounds");
    }
    for y in py..py + height {
        for x in px..px + width {
            unsafe { unchecked_draw_point(buf, x, y, color) };
        }
    }
    Ok(())
}

#[unsafe(no_mangle)]
fn efi_main(
    _image_handle: EfiHandle,
    efi_system_table: &EfiSystemTable,
) -> ! {

    let mut vram = init_vram(efi_system_table).expect("Failed to initialize VRAM");
    let vw = vram.width();
    let vh = vram.height();
    fill_rect(&mut vram, 0, 0, vw, vh, 0x000000).expect("Failed to fill rect");
    fill_rect(&mut vram, 32, 32, 32, 32, 0xff0000).expect("Failed to fill rect");
    fill_rect(&mut vram, 64, 64, 64, 64, 0x00ff00).expect("Failed to fill rect");
    fill_rect(&mut vram, 128, 128, 128, 128, 0x0000ff).expect("Failed to fill rect");
    for i in 0..256 {
        let _ = draw_point(&mut vram, i, i, 0x010101 * i as u32);
    }

    //println!("Hello, world!");

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
