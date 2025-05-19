use core::fmt;
use crate::graphics::draw_font_fg;
use crate::graphics::Bitmap;
use crate::result::Result;
use core::mem::size_of;
use core::ptr::null_mut;

pub type EfiHandle = u64;
type EfiVoid = u8;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct EfiGuid {
    data0: u32,
    data1: u16,
    data2: u16,
    data3: [u8; 8],
}

const EFI_GRAPHGICS_OUTPUT_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data0: 0x9042a9de,
    data1: 0x23dc,
    data2: 0x4a38,
    data3: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

#[derive(Debug, PartialEq, Eq,Clone, Copy)]
#[must_use]
#[repr(u64)]
pub enum EfiStatus {
    Success = 0,
}

#[repr(i64)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(non_camel_case_types)]
pub enum EfiMemoryType {
    RESERVED = 0,
    LOADER_CODE,
    LOADER_DATA,
    BOOT_SERVICES_CODE,
    BOOT_SERVICES_DATA,
    RUNTIME_SERVICES_CODE,
    RUNTIME_SERVICES_DATA,
    CONVENTIONAL_MEMORY,
    UNUSABLE_MEMORY,
    ACPI_RECLAIM_MEMORY,
    ACPI_NVS_MEMORY,
    MEMORY_MAPPED_IO,
    MEMORY_MAPPED_IO_PORT_SPACE,
    PAL_CODE,
    PERSISTENT_MEMORY,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MemoryDescriptor {
    memory_type: EfiMemoryType,
    physical_start: u64,
    virtual_start: u64,
    number_of_pages: u64,
    attribute: u64,
}

impl MemoryDescriptor {
    pub fn memory_type(&self) -> EfiMemoryType {
        self.memory_type
    }

    pub fn number_of_pages(&self) -> u64 {
        self.number_of_pages
    }

    pub fn physical_start(&self) -> u64 {
        self.physical_start
    }
}

const MEMORY_MAP_BUFFER_SIZE: usize = 0x8000;

pub struct MemoryMapHolder {
    buffer: [u8; MEMORY_MAP_BUFFER_SIZE],
    size: usize,
    map_key: usize,
    descriptor_size: usize,
    descriptor_version: u32,
}

impl MemoryMapHolder {
    pub const fn new() -> MemoryMapHolder {
        MemoryMapHolder {
            buffer: [0; MEMORY_MAP_BUFFER_SIZE],
            size: MEMORY_MAP_BUFFER_SIZE,
            map_key: 0,
            descriptor_size: 0,
            descriptor_version: 0,
        }
    }

    pub fn iter(&self) -> MemoryMapIterator {
        MemoryMapIterator {
            map: self,
            ofs: 0,
        }
    }
}

impl Default for MemoryMapHolder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MemoryMapIterator<'a> {
    map: &'a MemoryMapHolder,
    ofs: usize,
}

impl<'a> Iterator for MemoryMapIterator<'a>{
    type Item = &'a MemoryDescriptor;
    fn next(&mut self) -> Option<&'a MemoryDescriptor> {
        if self.ofs >= self.map.size {
            return None;
        }
        else {
            let e: &MemoryDescriptor = unsafe {
                &*(self.map.buffer.as_ptr().add(self.ofs) as *const MemoryDescriptor)
            };
            self.ofs += self.map.descriptor_size;
            Some(e)
        }
    }
}

#[repr(C)]
pub struct EfiBootServicesTable {
    _reserved0: [u64; 7],
    get_memory_map: extern "win64" fn(
        memory_map_size: *mut usize,
        memory_map: *mut u8,
        map_key: *mut usize,
        descripter_size: *mut usize,
        descriptor_version: *mut u32,
    ) -> EfiStatus,
    _reserved1: [u64; 21],
    exit_boot_services: extern "win64" fn(
        image_handle: EfiHandle,
        map_key: usize,
    ) -> EfiStatus,
    _reserved4: [u64; 10],
    locate_protocol: extern "win64" fn(
        protocol: *const EfiGuid,
        registration: *const EfiVoid,
        interface: *mut *mut EfiVoid,
    ) -> EfiStatus,
}

impl EfiBootServicesTable{
   pub fn get_memory_map(
        &self,
        map: &mut MemoryMapHolder,
    ) -> EfiStatus {
        (self.get_memory_map)(
            &mut map.size,
            map.buffer.as_mut_ptr(),
            &mut map.map_key,
            &mut map.descriptor_size,
            &mut map.descriptor_version,
        )
    }
}

#[repr(C)]
pub struct EfiSystemTable {
    _reserved0: [u64; 12],
    boot_services: &'static EfiBootServicesTable,
}

impl EfiSystemTable {
    pub fn boot_services(&self) -> &EfiBootServicesTable {
        self.boot_services
    }
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

#[derive(Clone, Copy)]
pub struct VramBufferInfo {
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

pub fn init_vram(
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

pub struct VramTextWriter<'a> {
    vram: &'a mut VramBufferInfo,
    curor_x: i64,
    curor_y: i64,
}

impl<'a> VramTextWriter<'a> {
   pub fn new(vram: &'a mut VramBufferInfo) -> Self {
        Self { vram,
            curor_x: 0,
            curor_y: 0 }
    }
}

impl fmt::Write for VramTextWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            if c == '\n' {
                self.curor_x = 0;
                self.curor_y += 16;
                continue;
            } 
            draw_font_fg(self.vram, self.curor_x, self.curor_y, 0xffffff, c);
            self.curor_x += 8;
        }
        Ok(())
    }
}

pub fn exit_from_efi_boot_services(
    image_handle: EfiHandle,
    efi_system_table: &EfiSystemTable,
    memory_map: &mut MemoryMapHolder,
) {
    loop {
        let status = efi_system_table.boot_services.get_memory_map(memory_map);
        assert_eq!(status, EfiStatus::Success);
        let status = (efi_system_table.boot_services.exit_boot_services)(
            image_handle,
            memory_map.map_key,
        );
        if status == EfiStatus::Success {
            break;
        }
    }
}