//! Spike S4: core <-> UI IPC. Throwaway code (D65); see spikes/s4-ipc/README.md and
//! docs/spikes/S4.md.

pub mod generic;
pub mod generic;
pub mod keyed;
pub mod scenario;
pub mod typed;
pub mod vm;

#[cfg(test)]
mod testgen;
