use crate::result::Result;
use core::cmp::min;

pub trait Bitmap {
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
    color: u32,
    x: i64,
    y: i64,
) -> Result<()> {
    *(buf.pixel_at_mut(x, y).ok_or("Out of bounds")?) = color;
    Ok(())
}

pub fn fill_rect<T: Bitmap>(
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

fn calc_slope_point(
    da: i64,
    db: i64,
    ia: i64,
) -> Option<i64> {
    if da < db{ 
        None
    } else if da == 0 {
        Some(0)
    } else if (0..=da).contains(&ia) {
        Some((2 * db * ia + da) / da / 2)
    } else {
        None
    }
}

fn draw_line<T: Bitmap>(
    buf: &mut T,
    color: u32,
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
) -> Result<()> {
    if !buf.is_in_x_range(x0)
        || !buf.is_in_x_range(x1)
        || !buf.is_in_y_range(y0)
        || !buf.is_in_y_range(y1)
    {
        return Err("Out of bounds");
    }
    let dx = (x1 - x0).abs();
    let sx = (x1 - x0).signum();
    let dy = (y1 - y0).abs();
    let sy = (y1 - y0).signum();
    if dx >= dy {
        for (rx, ry) in (0..dx)
            .flat_map(|rx| calc_slope_point(dx, dy, rx).map(|ry| (rx, ry))){
                draw_point(buf, color,x0 + rx * sx, y0 + ry * sy)?;
            }
    } else {
        for (rx, ry) in (0..dy)
            .flat_map(|ry| calc_slope_point(dy, dx, ry).map(|rx| (rx, ry))) {
                draw_point(buf, color,x0 + rx * sx, y0 + ry * sy)?;
            }
    }
    Ok(())    
}

fn lookup_font(
    c: char,
) -> Option<[[char; 8]; 16]>{
    const FONT_SOURCE: &str = include_str!("./font.txt");
    if let Ok(c) = u8::try_from(c) {
        let mut fi = FONT_SOURCE.split('\n');
        while let Some(line) = fi.next() {
            if let Some(line) = line.strip_prefix("0x") {
                if let Ok(idx) = u8::from_str_radix(line, 16) {
                    if idx != c {
                        continue;
                    }
                    let mut font = [['*'; 8]; 16];
                    for (y, line) in fi.clone().take(16).enumerate() {
                        for (x, c) in line.chars().enumerate() {
                            if let Some(e) = font[y].get_mut(x) {
                                *e = c;
                            }
                        }
                    }
                    return Some(font);
                }
            }
        }
    }
    None
}

pub fn draw_font_fg<T: Bitmap>(
    buf: &mut T,
    x: i64,
    y: i64,
    color: u32,
    c: char,
) {
    if let Some(font) = lookup_font(c) {
                for (dy,row) in font.iter().enumerate() {
                    for (dx, pixel) in row.iter().enumerate() {
                        let color = match pixel {
                            '*' => color,
                            _ => continue,
                        };
                        let _ = draw_point(buf, color, x + dx as i64, y + dy as i64);
                    }
                }
            }
}

pub fn draw_str_fg<T: Bitmap>(
    buf: &mut T,
    x: i64,
    y: i64,
    color: u32,
    s: &str,
) {
    for (i, c) in s.chars().enumerate() {
        draw_font_fg(buf, x + i as i64 * 8, y, color, c);
    }
}

pub fn draw_test_pattern<T: Bitmap>(buf: &mut T) {
    let w = 128;
    let left = buf.width() - w - 1;
    let colors = [ 0x000000, 0xff0000, 0x00ff00, 0x0000ff, 0xffff00, 0xff00ff, 0x00ffff, 0xffffff ];
    let h = 64;
    for (i, c) in colors.iter().enumerate() {
        let y = i as i64 * h;
        fill_rect(buf, left, y, h, h, *c).expect("Failed to fill rect");
        fill_rect(buf, left + h, y, h, h, !*c).expect("Failed to fill rect");
    }
    let points = [(0,0), (0,w), (w,0), (w,w)];
    for (x0,y0) in points.iter() {
        for (x1,y1) in points.iter() {
            draw_line(buf, 0xffffff, left + *x0, *y0, left + *x1, *y1).expect("Failed to draw line");
        }
    }
    draw_str_fg(buf, left, h * colors.len() as i64, 0x00ff00, "0123456789");
    draw_str_fg(buf, left, h * colors.len() as i64 + 16, 0x00ff00, "ABCDEF");
}