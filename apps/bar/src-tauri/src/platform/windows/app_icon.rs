//! An app's icon and starting it, both by its app user model id (AUMID, the id toasts carry).
//! Works for packaged and unpackaged apps: both show up in the `shell:AppsFolder` virtual folder.

use windows::core::HSTRING;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{DeleteObject, HGDIOBJ};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_ContainerFormatPng, GUID_WICPixelFormat32bppBGRA,
    IWICImagingFactory, WICBitmapEncoderNoCache, WICBitmapUseAlpha,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED, STATFLAG_NONAME,
    STATSTG, STREAM_SEEK_SET,
};
use windows::Win32::System::Com::StructuredStorage::CreateStreamOnHGlobal;
use windows::Win32::System::Com::IStream;
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, ShellExecuteW, SIIGBF_ICONONLY,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// Icons are drawn at 16 px in the panel; 32 stays sharp on a high-DPI screen.
const SIZE_PX: i32 = 32;

/// An id we are willing to hand to the shell: printable, no quotes, a sane length.
pub fn valid_app_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 256
        && !id.chars().any(|c| c.is_control() || c == '"' || c == '|')
}

fn ensure_com() {
    thread_local!(static DONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });
    DONE.with(|done| {
        if !done.get() {
            done.set(true);
            // SAFETY: plain COM init. If the thread already has an apartment this just fails,
            // which is fine: COM works in the one it has.
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();
            }
        }
    });
}

/// The app's icon as PNG bytes, or `None` if the shell has no such app.
pub fn icon_png(app_id: &str) -> Option<Vec<u8>> {
    if !valid_app_id(app_id) {
        return None;
    }
    ensure_com();
    // SAFETY: COM calls on this thread with values that live for the whole call. The bitmap the
    // shell gives us is ours to delete, which happens once, right after it is copied.
    unsafe {
        let path = HSTRING::from(format!("shell:AppsFolder\\{app_id}"));
        let factory: IShellItemImageFactory = SHCreateItemFromParsingName(&path, None).ok()?;
        let hbitmap = factory
            .GetImage(
                SIZE {
                    cx: SIZE_PX,
                    cy: SIZE_PX,
                },
                SIIGBF_ICONONLY,
            )
            .ok()?;
        let png = encode_png(hbitmap);
        let _ = DeleteObject(HGDIOBJ(hbitmap.0));
        png
    }
}

unsafe fn encode_png(hbitmap: windows::Win32::Graphics::Gdi::HBITMAP) -> Option<Vec<u8>> {
    let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_ALL).ok()?;
    let bitmap = wic
        .CreateBitmapFromHBITMAP(hbitmap, Default::default(), WICBitmapUseAlpha)
        .ok()?;
    let (mut w, mut h) = (0u32, 0u32);
    bitmap.GetSize(&mut w, &mut h).ok()?;
    let stream: IStream = CreateStreamOnHGlobal(Default::default(), true).ok()?;
    let encoder = wic.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null()).ok()?;
    encoder.Initialize(&stream, WICBitmapEncoderNoCache).ok()?;
    let mut frame = None;
    encoder.CreateNewFrame(&mut frame, std::ptr::null_mut()).ok()?;
    let frame = frame?;
    frame.Initialize(None).ok()?;
    frame.SetSize(w, h).ok()?;
    let mut format = GUID_WICPixelFormat32bppBGRA;
    frame.SetPixelFormat(&mut format).ok()?;
    frame.WriteSource(&bitmap, std::ptr::null()).ok()?;
    frame.Commit().ok()?;
    encoder.Commit().ok()?;

    let mut stat = STATSTG::default();
    stream.Stat(&mut stat, STATFLAG_NONAME).ok()?;
    let len = stat.cbSize as usize;
    stream.Seek(0, STREAM_SEEK_SET, None).ok()?;
    let mut out = vec![0u8; len];
    let mut read = 0u32;
    stream
        .Read(out.as_mut_ptr().cast(), len as u32, Some(&mut read))
        .ok()
        .ok()?;
    out.truncate(read as usize);
    Some(out)
}

/// Starts the app, or brings it to the front if it is running (what the shell does for it).
pub fn launch(app_id: &str) {
    if !valid_app_id(app_id) {
        return;
    }
    let target = HSTRING::from(format!("shell:AppsFolder\\{app_id}"));
    // SAFETY: strings live for the call; no window handle, no working directory.
    unsafe {
        ShellExecuteW(
            None,
            &HSTRING::from("open"),
            &target,
            None,
            None,
            SW_SHOWNORMAL,
        );
    }
}

/// Opens a `ms-settings:` page or another URI the shell knows.
pub fn open_uri(uri: &str) {
    // SAFETY: as above.
    unsafe {
        ShellExecuteW(
            None,
            &HSTRING::from("open"),
            &HSTRING::from(uri),
            None,
            None,
            SW_SHOWNORMAL,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_ids_with_odd_characters_are_refused() {
        assert!(valid_app_id("Microsoft.Todos_8wekyb3d8bbwe!App"));
        assert!(valid_app_id(r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe"));
        assert!(!valid_app_id(""));
        assert!(!valid_app_id("a\"b"));
        assert!(!valid_app_id("a\nb"));
        assert!(!valid_app_id(&"x".repeat(300)));
    }

    /// Needs a desktop with Windows PowerShell: it is in the apps folder with this id.
    #[test]
    #[ignore = "needs a desktop session"]
    fn powershell_has_an_icon() {
        let png = icon_png(r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe")
            .expect("icon");
        assert_eq!(&png[1..4], b"PNG");
    }
}
