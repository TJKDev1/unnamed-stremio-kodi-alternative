//! Spike S4: core <-> UI IPC. Throwaway code (D65); see spikes/s4-ipc/README.md and
//! docs/spikes/S4.md.

pub mod generic;
pub mod keyed;
pub mod scenario;
pub mod server;
pub mod session;
pub mod typed;
pub mod vm;
pub mod wire;

#[cfg(test)]
mod testgen;

/// p50, p95 and max of `xs` (sorted in place).
pub fn percentiles(xs: &mut [f64]) -> (f64, f64, f64) {
    if xs.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    xs.sort_by(f64::total_cmp);
    let at = |q: f64| xs[((xs.len() - 1) as f64 * q).round() as usize];
    (at(0.5), at(0.95), xs[xs.len() - 1])
}
