//! Decodes files through the *registered* WIC decoder using native WIC (no format conversion) and compares the
//! pixels byte-for-byte with `<name>.raw` (production Rust decoder output).  Usage: wictest <file.pkon>...
use windows::core::{Interface, HSTRING};
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::*;

fn main() {
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap() };
    let factory: IWICImagingFactory = unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap() };
    let mut bad = 0;
    for p in std::env::args().skip(1) {
        let r = (|| -> windows::core::Result<(u32, u32, windows::core::GUID, Vec<u8>, windows::core::GUID)> {
            let dec = unsafe { factory.CreateDecoderFromFilename(&HSTRING::from(p.as_str()), None, windows::Win32::Foundation::GENERIC_READ, WICDecodeMetadataCacheOnDemand)? };
            let container = unsafe { dec.GetContainerFormat()? };
            let f = unsafe { dec.GetFrame(0)? };
            let (mut w, mut h) = (0, 0);
            unsafe { f.GetSize(&mut w, &mut h)? };
            let fmt = unsafe { f.GetPixelFormat()? };
            let bpp = unsafe { factory.CreateComponentInfo(&fmt)?.cast::<IWICPixelFormatInfo>()?.GetBitsPerPixel()? };
            let stride = (w * bpp + 7) / 8;
            let mut buf = vec![0u8; (stride * h) as usize];
            unsafe { f.CopyPixels(std::ptr::null(), stride, &mut buf)? };
            Ok((w, h, fmt, buf, container))
        })();
        match r {
            Ok((w, h, fmt, buf, container)) => {
                let raw = std::fs::read(p.replace(".pkon", ".raw")).unwrap();
                let same = raw == buf;
                println!("{p}: {w}x{h} fmt={fmt:?} container={container:?} len={} exact_match={same}", buf.len());
                if !same { bad += 1 }
            }
            Err(e) => println!("{p}: ERROR {e}"),
        }
    }
    std::process::exit(bad);
}
