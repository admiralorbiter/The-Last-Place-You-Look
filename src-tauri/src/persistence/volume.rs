use crate::errors::AppError;

#[cfg(windows)]
pub fn resolve_volume_guid(path: &std::path::Path) -> Result<String, AppError> {
    use windows::core::PWSTR;
    use windows::Win32::Storage::FileSystem::GetVolumeNameForVolumeMountPointW;

    // Path must end with a backslash for the Win32 API
    let mut path_str = path.to_string_lossy().to_string();
    if !path_str.ends_with('\\') {
        path_str.push('\\');
    }

    let wide: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();
    let mut buf = vec![0u16; 50]; // Volume GUID format is fixed length

    unsafe {
        GetVolumeNameForVolumeMountPointW(
            windows::core::PCWSTR(wide.as_ptr()),
            &mut buf,
        ).map_err(|e| AppError::PlatformError(e.to_string()))?;
    }

    let guid = String::from_utf16_lossy(&buf)
        .trim_end_matches('\0')
        .to_string();

    Ok(guid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    #[cfg(windows)]
    fn test_resolve_c_drive_guid() {
        let guid = resolve_volume_guid(Path::new("C:\\")).unwrap();
        assert!(guid.starts_with("\\\\?\\Volume{"));
        assert!(guid.ends_with("}\\"));
    }
}
