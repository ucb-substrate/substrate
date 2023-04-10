//! Deserialization for Hammer-generated power strap JSON files.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::PowerStrapError;
use crate::error::{with_err_context, ErrorContext, SubstrateError};

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct StrapInfo<'a> {
    pub layer: &'a str,
    pub direction: &'a str,
    pub net_order: Vec<&'a str>,
    pub width: i64,
    pub spacing: i64,
    pub group_pitch: i64,
    #[serde(default)]
    pub offset: i64,
    pub inst_paths: Vec<&'a str>,
    pub inst_orientations: Vec<&'a str>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MacroConfig<'a> {
    /// Maps macro name to list of strap info, one per relevant layer.
    ///
    /// The map is expected to only ever contain one entry.
    #[serde(borrow)]
    config: HashMap<&'a str, Vec<StrapInfo<'a>>>,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HammerPowerStraps<'a> {
    /// Maps macro name to list of strap info, one per relevant layer.
    #[serde(borrow)]
    macros: Vec<MacroConfig<'a>>,
}

impl<'a> HammerPowerStraps<'a> {
    pub fn from_json(json: &'a str) -> crate::error::Result<Self> {
        let inner = || -> crate::error::Result<Self> {
            let val = serde_json::from_str(json)?;
            Ok(val)
        };
        with_err_context(inner(), || {
            ErrorContext::Task(arcstr::literal!(
                "parsing Hammer power strap configuration from JSON"
            ))
        })
    }

    pub(crate) fn get_macro(&self, name: &str) -> crate::error::Result<&Vec<StrapInfo<'a>>> {
        self.macros
            .iter()
            .flat_map(|m| m.config.iter())
            .find_map(|(macro_name, info)| {
                if *macro_name == name {
                    Some(info)
                } else {
                    None
                }
            })
            .ok_or_else(|| SubstrateError::new(PowerStrapError::HammerMacroNotFound(name.into())))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::tests::DATA_DIR;

    #[test]
    fn parse_hammer_power_straps() {
        let path = PathBuf::from(DATA_DIR).join("hammer/power_straps.json");
        let json = crate::io::read_to_string(path).unwrap();
        let straps = HammerPowerStraps::from_json(&json).unwrap();
        assert_eq!(straps.macros.len(), 4);

        assert_eq!(
            straps.macros[0].config["macro"],
            vec![
                StrapInfo {
                    layer: "M4",
                    direction: "horizontal",
                    net_order: vec!["VSS", "VDD"],
                    width: 864,
                    spacing: 288,
                    group_pitch: 10752,
                    offset: 3056,
                    inst_paths: vec!["pass/macro", "pass/macro_on_pitch"],
                    inst_orientations: vec!["r0", "r0"],
                },
                StrapInfo {
                    layer: "M5",
                    direction: "vertical",
                    net_order: vec!["VSS", "VDD"],
                    width: 864,
                    spacing: 288,
                    group_pitch: 10752,
                    offset: 0,
                    inst_paths: vec!["pass/macro", "pass/macro_on_pitch"],
                    inst_orientations: vec!["r0", "r0"],
                }
            ]
        );
    }
}
