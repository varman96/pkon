//! WIC decoder adapter for `.pkon` (PKON2 v1) files. All decoding is `nanoscope::pkonfile::decode_with_limit`;
//! this crate only validates the stream size against the header, maps errors to WINCODEC_ERR_* codes, and exposes
//! the decoded 8-bit pixels unchanged as a single-frame WIC bitmap.
//!
//! Pixel formats: Gray -> 8bppGray, Rgb -> 24bppRGB, Rgba -> 32bppRGBA (straight alpha). GrayAlpha has no WIC
//! pixel format, so it is rejected with WINCODEC_ERR_UNSUPPORTEDPIXELFORMAT rather than converted.
use nanoscope::canon::Layout;
use nanoscope::pkonfile::{decode_with_limit, parse_header_with_limit, PkonError, DEFAULT_MAX_DECODED_BYTES, HEADER_LEN};
use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use windows::core::{implement, Error, Interface, Ref, Result, BOOL, GUID, HRESULT};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::*;

/// CLSID of the decoder (registered under HKCR\CLSID and the WIC decoder category).
pub const CLSID_PKON_DECODER: GUID = GUID::from_u128(0xa1d6f3e2_8b47_4f09_b5c3_2e9d7a10c648);
/// Container format GUID identifying PKON2 files to WIC.
pub const GUID_CONTAINER_PKON: GUID = GUID::from_u128(0x7b3c5a0e_4d1f_4c8a_9b62_5e0a1f3d7c24);

fn hr(code: HRESULT) -> Error {
    Error::from(code)
}

fn map_err(e: &PkonError) -> Error {
    use PkonError::*;
    hr(match e {
        Truncated { .. } => WINCODEC_ERR_STREAMREAD,
        BadMagic | UnsupportedVersion(_) | ReservedFlags(_) | InvalidDimensions { .. } | InvalidPayloadLength { .. } => WINCODEC_ERR_BADHEADER,
        UnsupportedCodec(_) => WINCODEC_ERR_UNSUPPORTEDOPERATION,
        UnsupportedPixelFormat(_) | UnsupportedImage(_) => WINCODEC_ERR_UNSUPPORTEDPIXELFORMAT,
        TooLarge { .. } => WINCODEC_ERR_IMAGESIZEOUTOFRANGE,
        TrailingBytes(_) | CorruptPayload | ChecksumMismatch { .. } => WINCODEC_ERR_BADIMAGE,
    })
}

struct Frame {
    width: u32,
    height: u32,
    bpp: u32,
    format: GUID,
    data: Vec<u8>,
}

fn read_exact(stream: &IStream, buf: &mut [u8]) -> Result<()> {
    let mut done = 0usize;
    while done < buf.len() {
        let mut n = 0u32;
        let want = (buf.len() - done).min(1 << 20) as u32;
        unsafe { stream.Read(buf[done..].as_mut_ptr() as *mut c_void, want, Some(&mut n)).ok()? };
        if n == 0 {
            return Err(hr(WINCODEC_ERR_STREAMREAD));
        }
        done += n as usize;
    }
    Ok(())
}

fn stream_size(stream: &IStream) -> Result<u64> {
    let mut st = STATSTG::default();
    unsafe { stream.Stat(&mut st, STATFLAG_NONAME)? };
    Ok(st.cbSize)
}

fn rewind(stream: &IStream) -> Result<()> {
    unsafe { stream.Seek(0, STREAM_SEEK_SET, None) }
}

/// Read and validate the header only (24 bytes); nothing is allocated from header-claimed sizes.
fn read_header(stream: &IStream) -> Result<(nanoscope::pkonfile::Header, u64)> {
    rewind(stream)?;
    let size = stream_size(stream)?;
    let take = size.min(HEADER_LEN as u64) as usize;
    let mut h = [0u8; HEADER_LEN];
    read_exact(stream, &mut h[..take])?;
    let hdr = parse_header_with_limit(&h[..take], DEFAULT_MAX_DECODED_BYTES).map_err(|e| map_err(&e))?;
    Ok((hdr, size))
}

fn load(stream: &IStream) -> Result<Frame> {
    let (hdr, size) = read_header(stream)?;
    let end = HEADER_LEN as u64 + hdr.payload_len as u64;
    // The stream size is checked against the header before the payload buffer is allocated.
    if size < end {
        return Err(map_err(&PkonError::Truncated { need: end as usize, have: size as usize }));
    }
    if size > end {
        return Err(map_err(&PkonError::TrailingBytes((size - end) as usize)));
    }
    let (format, bpp) = match hdr.spec.layout {
        Layout::Gray => (GUID_WICPixelFormat8bppGray, 8),
        Layout::Rgb => (GUID_WICPixelFormat24bppRGB, 24),
        Layout::Rgba => (GUID_WICPixelFormat32bppRGBA, 32),
        Layout::GrayAlpha => return Err(hr(WINCODEC_ERR_UNSUPPORTEDPIXELFORMAT)),
    };
    rewind(stream)?;
    let mut bytes = vec![0u8; end as usize];
    read_exact(stream, &mut bytes)?;
    let img = decode_with_limit(&bytes, DEFAULT_MAX_DECODED_BYTES).map_err(|e| map_err(&e))?;
    Ok(Frame { width: img.spec.width, height: img.spec.height, bpp, format, data: img.data })
}

#[implement(IWICBitmapDecoder)]
struct Decoder {
    frame: Mutex<Option<Arc<Frame>>>,
}

impl IWICBitmapDecoder_Impl for Decoder_Impl {
    fn QueryCapability(&self, pistream: Ref<'_, IStream>) -> Result<u32> {
        let s = pistream.ok()?;
        load(s)?;
        Ok((WICBitmapDecoderCapabilityCanDecodeAllImages.0 | WICBitmapDecoderCapabilityCanDecodeSomeImages.0) as u32)
    }
    fn Initialize(&self, pistream: Ref<'_, IStream>, _cacheoptions: WICDecodeOptions) -> Result<()> {
        let f = load(pistream.ok()?)?;
        *self.frame.lock().unwrap() = Some(Arc::new(f));
        Ok(())
    }
    fn GetContainerFormat(&self) -> Result<GUID> {
        Ok(GUID_CONTAINER_PKON)
    }
    fn GetDecoderInfo(&self) -> Result<IWICBitmapDecoderInfo> {
        let factory: IWICImagingFactory = unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)? };
        unsafe { factory.CreateComponentInfo(&CLSID_PKON_DECODER)?.cast() }
    }
    fn CopyPalette(&self, _pipalette: Ref<'_, IWICPalette>) -> Result<()> {
        Err(hr(WINCODEC_ERR_PALETTEUNAVAILABLE))
    }
    fn GetMetadataQueryReader(&self) -> Result<IWICMetadataQueryReader> {
        Err(hr(WINCODEC_ERR_UNSUPPORTEDOPERATION))
    }
    fn GetPreview(&self) -> Result<IWICBitmapSource> {
        Err(hr(WINCODEC_ERR_UNSUPPORTEDOPERATION))
    }
    fn GetColorContexts(&self, _ccount: u32, _ppicolorcontexts: *mut Option<IWICColorContext>, pcactualcount: *mut u32) -> Result<()> {
        if !pcactualcount.is_null() {
            unsafe { *pcactualcount = 0 };
        }
        Ok(())
    }
    fn GetThumbnail(&self) -> Result<IWICBitmapSource> {
        Err(hr(WINCODEC_ERR_CODECNOTHUMBNAIL))
    }
    fn GetFrameCount(&self) -> Result<u32> {
        Ok(1)
    }
    fn GetFrame(&self, index: u32) -> Result<IWICBitmapFrameDecode> {
        let f = self.frame.lock().unwrap().clone().ok_or_else(|| hr(WINCODEC_ERR_NOTINITIALIZED))?;
        if index != 0 {
            return Err(hr(WINCODEC_ERR_FRAMEMISSING));
        }
        Ok(FrameDecode { frame: f }.into())
    }
}

#[implement(IWICBitmapFrameDecode)]
struct FrameDecode {
    frame: Arc<Frame>,
}

impl IWICBitmapSource_Impl for FrameDecode_Impl {
    fn GetSize(&self, puiwidth: *mut u32, puiheight: *mut u32) -> Result<()> {
        unsafe {
            *puiwidth = self.frame.width;
            *puiheight = self.frame.height;
        }
        Ok(())
    }
    fn GetPixelFormat(&self) -> Result<GUID> {
        Ok(self.frame.format)
    }
    fn GetResolution(&self, pdpix: *mut f64, pdpiy: *mut f64) -> Result<()> {
        unsafe {
            *pdpix = 96.0;
            *pdpiy = 96.0;
        }
        Ok(())
    }
    fn CopyPalette(&self, _pipalette: Ref<'_, IWICPalette>) -> Result<()> {
        Err(hr(WINCODEC_ERR_PALETTEUNAVAILABLE))
    }
    fn CopyPixels(&self, prc: *const WICRect, cbstride: u32, cbbuffersize: u32, pbbuffer: *mut u8) -> Result<()> {
        let f = &*self.frame;
        let (x, y, w, h) = if prc.is_null() {
            (0u64, 0u64, f.width as u64, f.height as u64)
        } else {
            let r = unsafe { *prc };
            if r.X < 0 || r.Y < 0 || r.Width < 0 || r.Height < 0 {
                return Err(hr(HRESULT(WINCODEC_ERR_INVALIDPARAMETER)));
            }
            (r.X as u64, r.Y as u64, r.Width as u64, r.Height as u64)
        };
        if x + w > f.width as u64 || y + h > f.height as u64 {
            return Err(hr(HRESULT(WINCODEC_ERR_INVALIDPARAMETER)));
        }
        if w == 0 || h == 0 {
            return Ok(());
        }
        if pbbuffer.is_null() {
            return Err(E_POINTER.into());
        }
        let bytes_pp = (f.bpp / 8) as u64;
        let row_bytes = w * bytes_pp;
        if (cbstride as u64) < row_bytes || (cbbuffersize as u64) < (h - 1) * cbstride as u64 + row_bytes {
            return Err(hr(WINCODEC_ERR_INSUFFICIENTBUFFER));
        }
        let src_stride = f.width as u64 * bytes_pp;
        for row in 0..h {
            let s = ((y + row) * src_stride + x * bytes_pp) as usize;
            unsafe {
                std::ptr::copy_nonoverlapping(f.data.as_ptr().add(s), pbbuffer.add((row * cbstride as u64) as usize), row_bytes as usize);
            }
        }
        Ok(())
    }
}

impl IWICBitmapFrameDecode_Impl for FrameDecode_Impl {
    fn GetMetadataQueryReader(&self) -> Result<IWICMetadataQueryReader> {
        Err(hr(WINCODEC_ERR_UNSUPPORTEDOPERATION))
    }
    fn GetColorContexts(&self, _ccount: u32, _ppicolorcontexts: *mut Option<IWICColorContext>, pcactualcount: *mut u32) -> Result<()> {
        if !pcactualcount.is_null() {
            unsafe { *pcactualcount = 0 };
        }
        Ok(())
    }
    fn GetThumbnail(&self) -> Result<IWICBitmapSource> {
        Err(hr(WINCODEC_ERR_CODECNOTHUMBNAIL))
    }
}

#[implement(IClassFactory)]
struct Factory;

impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(&self, punkouter: Ref<'_, windows::core::IUnknown>, riid: *const GUID, ppvobject: *mut *mut c_void) -> Result<()> {
        if !punkouter.is_null() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        let d: IWICBitmapDecoder = Decoder { frame: Mutex::new(None) }.into();
        unsafe { d.query(riid, ppvobject).ok() }
    }
    fn LockServer(&self, _flock: BOOL) -> Result<()> {
        Ok(())
    }
}

#[no_mangle]
pub extern "system" fn DllGetClassObject(rclsid: *const GUID, riid: *const GUID, ppv: *mut *mut c_void) -> HRESULT {
    unsafe {
        if ppv.is_null() || rclsid.is_null() || riid.is_null() {
            return E_POINTER;
        }
        *ppv = std::ptr::null_mut();
        if *rclsid != CLSID_PKON_DECODER {
            return CLASS_E_CLASSNOTAVAILABLE;
        }
        let f: IClassFactory = Factory.into();
        f.query(riid, ppv)
    }
}

/// The DLL stays loaded for the life of the process; WIC clients do not unload decoders.
#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    S_FALSE
}
