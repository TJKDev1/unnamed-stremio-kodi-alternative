//! Rust-side numbers for T0.S4.3: diff + encode time and bytes per frame, for every
//! scenario × diff × encoding. `cargo run --release --bin bench`

use s4_core::percentiles;
use s4_core::scenario::SCENARIOS;
use s4_core::session::{HEADER, Session};
use s4_core::wire::{DIFFS, ENCODINGS};

const RUNS: usize = 50;

fn main() {
    println!(
        "| scenario | diff | encoding | frames | diff µs p50/p95/max | encode µs p50/p95/max | bytes p50/max |"
    );
    println!("|---|---|---|---|---|---|---|");
    for name in SCENARIOS {
        for diff in DIFFS {
            for enc in ENCODINGS {
                let (mut d, mut e, mut b) = (Vec::new(), Vec::new(), Vec::new());
                let mut frames = 0;
                for _ in 0..RUNS {
                    let mut s = Session::new(name, diff, enc).unwrap();
                    frames = 0;
                    while let Some(f) = s.next_frame() {
                        frames += 1;
                        let u = |i: usize| {
                            u32::from_le_bytes(f[i * 4..i * 4 + 4].try_into().unwrap()) as f64
                                / 1000.0
                        };
                        d.push(u(1));
                        e.push(u(2));
                        b.push((f.len() - HEADER) as f64);
                    }
                }
                let (d50, d95, dmax) = percentiles(&mut d);
                let (e50, e95, emax) = percentiles(&mut e);
                let (b50, _, bmax) = percentiles(&mut b);
                println!(
                    "| {name} | {diff:?} | {enc:?} | {frames} | {d50:.1} / {d95:.1} / {dmax:.1} | {e50:.1} / {e95:.1} / {emax:.1} | {b50:.0} / {bmax:.0} |"
                );
            }
        }
    }
}
