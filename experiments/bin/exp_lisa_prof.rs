//! In-process sampling CPU profiler for the LISA encoder (Windows x64 only; no external dependencies).
//! A sampler thread repeatedly suspends the encoding thread, captures its register context, unwinds the stack with
//! RtlVirtualUnwind and records the return addresses. Samples are CPU-on-thread samples of the encoder thread only.
//! Output: one line per sample, "addr addr addr ..." (leaf first, absolute VAs), preceded by "#base <hex image base>".
//! Usage: exp_lisa_prof <canon.u16> <w> <h> <bit_depth> <out_samples.txt> [reps=1] [decode=0|1]
use nanoscope::lisa::*;
use nanoscope::lisa_bayer_plane::{decode_stage1, encode_stage2, restore_stage1_with_mode};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[repr(C, align(16))]
struct Context([u8; 1232]);

type Handle = isize;
extern "system" {
    fn GetCurrentThreadId() -> u32;
    fn OpenThread(access: u32, inherit: i32, tid: u32) -> Handle;
    fn SuspendThread(h: Handle) -> u32;
    fn ResumeThread(h: Handle) -> u32;
    fn GetThreadContext(h: Handle, ctx: *mut Context) -> i32;
    fn RtlLookupFunctionEntry(pc: u64, image_base: *mut u64, history: *mut u8) -> *mut u8;
    fn RtlVirtualUnwind(
        handler_type: u32, image_base: u64, pc: u64, fe: *mut u8, ctx: *mut Context, handler_data: *mut *mut u8, establisher: *mut u64,
        ctx_ptrs: *mut u8,
    ) -> *mut u8;
    fn GetModuleHandleW(name: *const u16) -> isize;
}

const RSP: usize = 0x98;
const RIP: usize = 0xF8;
fn rd(c: &Context, off: usize) -> u64 {
    u64::from_le_bytes(c.0[off..off + 8].try_into().unwrap())
}

fn unwind(ctx: &mut Context) -> Vec<u64> {
    let mut out = vec![];
    for _ in 0..48 {
        let pc = rd(ctx, RIP);
        if pc == 0 {
            break;
        }
        out.push(pc);
        let (mut base, mut hd, mut est) = (0u64, std::ptr::null_mut(), 0u64);
        let fe = unsafe { RtlLookupFunctionEntry(pc, &mut base, std::ptr::null_mut()) };
        if fe.is_null() {
            // Leaf function without unwind data: return address is at [rsp].
            let sp = rd(ctx, RSP);
            let ra = unsafe { *(sp as *const u64) };
            ctx.0[RIP..RIP + 8].copy_from_slice(&ra.to_le_bytes());
            ctx.0[RSP..RSP + 8].copy_from_slice(&(sp + 8).to_le_bytes());
        } else {
            unsafe { RtlVirtualUnwind(0, base, pc, fe, ctx, &mut hd, &mut est, std::ptr::null_mut()) };
        }
    }
    out
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (w, h): (usize, usize) = (a[2].parse().unwrap(), a[3].parse().unwrap());
    let bd: u8 = a[4].parse().unwrap();
    let reps: usize = a.get(6).map_or(1, |s| s.parse().unwrap());
    let decode = a.get(7).map_or(false, |s| s == "1");
    let samples: Vec<u16> = std::fs::read(&a[1]).unwrap().chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let meta = {
        // All corpus Bayer images use a 2x2 CFA; the exact colour order does not affect cost, but use RGGB for validity.
        let pat = CfaPattern { width: 2, height: 2, entries: vec![ColorFilter::Red, ColorFilter::Green, ColorFilter::Green, ColorFilter::Blue], origin_x: 0, origin_y: 0 };
        ImageMetadata::with_cfa(w, h, SampleLayout::BayerCfa, pat)
    };
    let tid = unsafe { GetCurrentThreadId() };
    let th = unsafe { OpenThread(0x4A, 0, tid) };
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let sampler = std::thread::spawn(move || {
        let mut all: Vec<Vec<u64>> = vec![];
        let mut next = Instant::now();
        while !stop2.load(Ordering::Relaxed) {
            next += Duration::from_micros(1000);
            while Instant::now() < next {
                std::hint::spin_loop();
            }
            unsafe {
                if SuspendThread(th) == u32::MAX {
                    continue;
                }
                let mut ctx = Context([0; 1232]);
                ctx.0[0x30..0x34].copy_from_slice(&0x10000Bu32.to_le_bytes());
                if GetThreadContext(th, &mut ctx) != 0 {
                    all.push(unwind(&mut ctx));
                }
                ResumeThread(th);
            }
        }
        all
    });
    let t = Instant::now();
    let mut s4 = None;
    for _ in 0..reps {
        let s2 = encode_stage2(&samples, w, h, bd, meta.clone()).unwrap();
        s4 = Some(encode_stage4(classify_stage3(s2).unwrap()).unwrap());
    }
    let enc_s = t.elapsed().as_secs_f64();
    let t = Instant::now();
    if decode {
        let s4 = s4.as_ref().unwrap();
        let d3 = decode_stage4(s4).unwrap();
        let (s1, mode) = restore_stage1_with_mode(&d3.into_stage2()).unwrap();
        assert!(decode_stage1(&s1, mode).unwrap() == samples);
    }
    let dec_s = t.elapsed().as_secs_f64();
    stop.store(true, Ordering::Relaxed);
    let all = sampler.join().unwrap();
    let base = unsafe { GetModuleHandleW(std::ptr::null()) } as u64;
    let mut f = std::io::BufWriter::new(std::fs::File::create(&a[5]).unwrap());
    writeln!(f, "#base {base:x}").unwrap();
    for s in &all {
        writeln!(f, "{}", s.iter().map(|x| format!("{x:x}")).collect::<Vec<_>>().join(" ")).unwrap();
    }
    eprintln!("samples={} enc_s={enc_s:.3} dec_s={dec_s:.3}", all.len());
}
