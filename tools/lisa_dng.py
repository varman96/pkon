"""DNG raw-mosaic reader for the LISA benchmark (no demosaic / WB / crop / scaling).

Returns the stored raster of the main raw IFD (CFA / LinearRaw / mono) as a uint16 array plus the metadata needed
to interpret it. Run with `python -I`."""
import numpy as np
import tifffile


def _all_pages(tf):
    out = []
    for p in tf.pages:
        out.append(p)
        subs = getattr(p, "pages", None)
        if subs:
            out.extend(subs)
    return out


def _val(p, name):
    t = p.tags.get(name)
    return None if t is None else t.value


def _list(v):
    if v is None:
        return None
    if isinstance(v, (bytes, bytearray)):
        return [int(x) for x in v]
    if isinstance(v, (tuple, list)):
        return [x if isinstance(x, (int, float, str)) else float(x) for x in v]
    return [v]


def raw_page(tf):
    pages = _all_pages(tf)
    raw = [p for p in pages if int(p.photometric) in (32803, 34892)]
    if not raw:  # monochrome DNG: largest page with a DNGVersion-bearing parent
        raw = [max(pages, key=lambda p: p.imagewidth * p.imagelength)]
    return max(raw, key=lambda p: p.imagewidth * p.imagelength)


def read_dng(path):
    with tifffile.TiffFile(path) as tf:
        p = raw_page(tf)
        root = tf.pages[0]
        comp = p.compression
        bps = p.bitspersample
        info = {
            "make": _val(root, "Make"), "model": _val(root, "Model"), "unique_model": _val(root, "UniqueCameraModel"),
            "width": int(p.imagewidth), "height": int(p.imagelength), "samples_per_pixel": int(p.samplesperpixel),
            "photometric": int(p.photometric), "compression": int(comp),
            "compression_name": getattr(comp, "name", str(comp)),
            "bits_per_sample": bps if isinstance(bps, int) else list(bps),
            "sample_format": [int(x) for x in np.atleast_1d(p.sampleformat)],
            "predictor": int(p.predictor), "tiled": bool(p.is_tiled),
            "cfa_pattern": _list(_val(p, "CFAPattern")), "cfa_dim": _list(_val(p, "CFARepeatPatternDim")),
            "cfa_layout": _list(_val(p, "CFALayout")),
            "white_level": _list(_val(p, "WhiteLevel")), "black_level": _list(_val(p, "BlackLevel")),
            "active_area": _list(_val(p, "ActiveArea")),
        }
        a = p.asarray()
        info["array_dtype"] = str(a.dtype)
        info["array_shape"] = list(a.shape)
        return info, a
