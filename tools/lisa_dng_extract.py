"""DNG -> canonical LISA input for the PKON converter UI (`src/dng.rs` runs this; run with `python -I`).

Reads a DNG from stdin through `lisa_dng.read_dng` (the same reader as `lisa_bench1_prepare.py`) and writes to stdout:
one line of compact JSON metadata, then the full stored raster as row-major little-endian uint16.
No ActiveArea/DefaultCrop crop, no black-level/white-balance/linearization/scaling/demosaic. Metadata derivation
(bit depth, CFA origin) is identical to `lisa_bench1_prepare.py`. Errors: message on stderr, exit status 2."""
import io, json, os, sys
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lisa_dng


def fail(msg):
    sys.stderr.write(msg)
    sys.exit(2)


def main():
    data = sys.stdin.buffer.read()
    try:
        info, a = lisa_dng.read_dng(io.BytesIO(data))
    except Exception as e:  # tifffile raises many types for malformed/unsupported files
        fail(f"cannot read DNG: {e}")
    if a.ndim != 2:
        fail(f"unsupported DNG: stored raster has shape {list(a.shape)} (only single-plane CFA/monochrome RAW is supported)")
    if a.dtype not in (np.uint8, np.uint16):
        fail(f"unsupported DNG: sample type {a.dtype} (only 8/16-bit unsigned integer samples are supported)")
    bps = info["bits_per_sample"]
    if not isinstance(bps, int):
        fail(f"unsupported DNG: bits per sample {bps}")
    a16 = np.ascontiguousarray(a.astype("<u2"))
    h, w = a16.shape
    mx = int(a16.max())
    wl = max(info["white_level"] or [0])
    bit_depth = min(bps, max(1, int(max(mx, wl)).bit_length()))
    if mx >= (1 << bit_depth):
        fail(f"unsupported DNG: sample value {mx} does not fit {bit_depth} bits")
    cfa, dim = info["cfa_pattern"], info["cfa_dim"]
    origin = None
    if cfa:
        aa = info["active_area"] or [0, 0, h, w]
        origin = [(-aa[1]) % dim[1], (-aa[0]) % dim[0]]  # pattern position of image sample (0,0), as [x, y]
    meta = dict(
        width=w, height=h, declared_bits=bps, bit_depth=bit_depth, cfa=cfa, cfa_dim=dim, cfa_origin=origin,
        camera=f'{info["make"] or ""} {info["model"] or ""}'.strip(), dng_compression=info["compression_name"],
    )
    out = sys.stdout.buffer
    out.write(json.dumps(meta, separators=(",", ":")).encode("ascii", "replace") + b"\n")
    out.write(a16.tobytes())
    out.flush()


main()
