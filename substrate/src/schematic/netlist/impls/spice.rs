//! A built-in SPICE netlister implementation.

use std::path::Path;

use crate::fmt::signal::format_signal;
use crate::schematic::netlist::interface::{
    InstanceInfo, NetlistOpts, Netlister, Result, SubcircuitInfo,
};

/// A SPICE netlister.
#[derive(Clone, Debug, Default)]
pub struct SpiceNetlister;

impl SpiceNetlister {
    /// Creates a new [`SpiceNetlister`].
    #[inline]
    pub fn new() -> Self {
        Self
    }
}

impl Netlister for SpiceNetlister {
    /// Returns configuration options for this netlister.
    ///
    /// The global ground net is named `0` by default.
    fn opts(&self) -> crate::schematic::netlist::interface::NetlistOpts {
        NetlistOpts {
            global_ground_net: arcstr::literal!("0"),
            ..Default::default()
        }
    }

    fn emit_comment(&self, out: &mut dyn std::io::Write, comment: &str) -> Result<()> {
        writeln!(out, "* {comment}")?;
        Ok(())
    }

    fn emit_begin_subcircuit(
        &self,
        out: &mut dyn std::io::Write,
        info: SubcircuitInfo,
    ) -> Result<()> {
        writeln!(out, "\n.subckt {}", info.name)?;
        for &port in info.ports {
            let sig = &info.signals[port.signal];
            for i in 0..sig.width() {
                writeln!(
                    out,
                    "+ {}",
                    format_signal(sig.name(), i, sig.width(), self.opts().bus_format)
                )?;
            }
        }
        // Write a newline
        writeln!(out)?;
        Ok(())
    }

    fn emit_end_subcircuit(&self, out: &mut dyn std::io::Write, name: &str) -> Result<()> {
        writeln!(out, ".ends {name}\n")?;
        Ok(())
    }

    fn emit_raw_spice(&self, out: &mut dyn std::io::Write, spice: &str) -> Result<()> {
        writeln!(out, "{spice}")?;
        Ok(())
    }

    fn emit_instance(&self, out: &mut dyn std::io::Write, instance: InstanceInfo) -> Result<()> {
        writeln!(out, "X{}", instance.name)?;
        for &signal in instance.ports {
            for part in signal.parts() {
                let info = &instance.signals[part.signal()];
                if info.width() == 1 {
                    writeln!(out, "+ {}", info.name())?;
                } else {
                    for i in part.range() {
                        writeln!(out, "+ {}[{}]", info.name(), i)?;
                    }
                }
            }
        }
        writeln!(out, "+ {}", instance.subcircuit_name)?;
        Ok(())
    }

    fn emit_include(&self, out: &mut dyn std::io::Write, include: &Path) -> Result<()> {
        writeln!(out, ".include {include:?}")?;
        Ok(())
    }

    fn emit_lib_include(
        &self,
        out: &mut dyn std::io::Write,
        lib: &Path,
        section: &str,
    ) -> Result<()> {
        writeln!(out, ".lib {lib:?} {section}")?;
        Ok(())
    }
}
