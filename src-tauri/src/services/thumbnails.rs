use std::path::{Path};
use windows::core::{PCWSTR, Interface};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use windows::Win32::UI::Shell::{SHCreateItemFromParsingName, IShellItem, IShellItemImageFactory, SIIGBF_RESIZETOFIT};
use windows::Win32::Graphics::Gdi::{HBITMAP, GetObjectW, GetDIBits, BITMAP, DIB_RGB_COLORS, BITMAPINFO, BITMAPINFOHEADER, BI_RGB};
use windows::Win32::Foundation::SIZE;

pub fn extract_thumbnail(path: &Path, size_px: i32) -> Result<Vec<u8>, String> {
    unsafe {
        // Initialize COM on this thread. We ignore errors since it might have already been initialized
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        
        let mut path_u16: Vec<u16> = path.to_string_lossy().encode_utf16().collect();
        path_u16.push(0);
        
        let shell_item: IShellItem = match SHCreateItemFromParsingName(PCWSTR(path_u16.as_ptr()), None) {
            Ok(item) => item,
            Err(e) => return Err(format!("SHCreateItemFromParsingName failed: {}", e)),
        };
        
        let image_factory: IShellItemImageFactory = match shell_item.cast() {
            Ok(f) => f,
            Err(e) => return Err(format!("Failed to cast to IShellItemImageFactory: {}", e)),
        };
        
        let size = SIZE { cx: size_px, cy: size_px };
        
        // Request the thumbnail from the shell
        let hbitmap = match image_factory.GetImage(size, SIIGBF_RESIZETOFIT) {
            Ok(h) => h,
            Err(e) => return Err(format!("GetImage failed: {}", e)),
        };
        
        // Convert HBITMAP to PNG bytes
        let bytes = bitmap_to_png(hbitmap);
        
        // Free the HBITMAP
        let _ = windows::Win32::Graphics::Gdi::DeleteObject(hbitmap);
        
        bytes
    }
}

unsafe fn bitmap_to_png(hbitmap: HBITMAP) -> Result<Vec<u8>, String> {
    let mut bm = BITMAP::default();
    if GetObjectW(hbitmap, std::mem::size_of::<BITMAP>() as i32, Some(&mut bm as *mut _ as *mut std::ffi::c_void)) == 0 {
        return Err("GetObjectW failed".into());
    }
    
    let width = bm.bmWidth;
    let height = bm.bmHeight;
    let mut pixels: Vec<u8> = vec![0; (width * height * 4) as usize];
    
    let mut bi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // negative means top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0 as u32,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [windows::Win32::Graphics::Gdi::RGBQUAD::default(); 1],
    };
    
    let hdc = windows::Win32::Graphics::Gdi::CreateCompatibleDC(None);
    if hdc.is_invalid() {
        return Err("CreateCompatibleDC failed".into());
    }
    
    let res = GetDIBits(hdc, hbitmap, 0, height as u32, Some(pixels.as_mut_ptr() as *mut _), &mut bi, DIB_RGB_COLORS);
    let _ = windows::Win32::Graphics::Gdi::DeleteDC(hdc);
    
    if res == 0 {
        return Err("GetDIBits failed".into());
    }
    
    // BGRA to RGBA (and fix premultiplied alpha on Windows DIB)
    for i in (0..pixels.len()).step_by(4) {
        let b = pixels[i];
        let r = pixels[i + 2];
        
        pixels[i] = r;
        pixels[i + 2] = b;
        pixels[i + 3] = 255; // Force full opacity; thumbnails don't need transparency
    }
    
    let mut png_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    if let Err(e) = image::write_buffer_with_format(
        &mut cursor,
        &pixels,
        width as u32,
        height as u32,
        image::ColorType::Rgba8,
        image::ImageFormat::Png
    ) {
        return Err(format!("Image encoding failed: {}", e));
    }
    
    Ok(png_bytes)
}
