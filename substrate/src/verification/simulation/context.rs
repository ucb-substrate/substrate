use std::path::PathBuf;

use super::{Analysis, OutputFormat, Save, SimInput, SimOutput};
use crate::units::SiValue;

pub struct PreSimCtx {
    pub(crate) input: SimInput,
}

pub struct PostSimCtx {
    pub(crate) output: SimOutput,
}

impl PreSimCtx {
    #[inline]
    pub(crate) fn new(input: SimInput) -> Self {
        Self { input }
    }

    pub fn add_analysis(&mut self, analysis: impl Into<Analysis>) -> &mut Self {
        self.input.analyses.push(analysis.into());
        self
    }

    pub fn save(&mut self, save: Save) -> &mut Self {
        self.input.save = save;
        self
    }

    pub fn include(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.input.includes.push(path.into());
        self
    }

    pub fn set_format(&mut self, format: impl Into<OutputFormat>) -> &mut Self {
        self.input.output_format = format.into();
        self
    }

    pub fn set_temp(&mut self, temp: f64) -> &mut Self {
        self.input.opts.temp = Some(temp);
        self
    }

    pub fn set_flags(&mut self, flags: impl Into<String>) -> &mut Self {
        self.input.opts.flags = Some(flags.into());
        self
    }

    pub fn include_lib(
        &mut self,
        path: impl Into<PathBuf>,
        section: impl Into<String>,
    ) -> &mut Self {
        self.input.libs.push(super::Lib {
            path: path.into(),
            section: section.into(),
        });
        self
    }

    pub fn set_ic(&mut self, node: impl Into<String>, value: SiValue) -> &mut Self {
        self.input.ic.insert(node.into(), value);
        self
    }

    #[inline]
    #[allow(unused)]
    pub(crate) fn inner(&self) -> &SimInput {
        &self.input
    }

    #[inline]
    #[allow(unused)]
    pub(crate) fn inner_mut(&mut self) -> &mut SimInput {
        &mut self.input
    }

    #[inline]
    pub(crate) fn into_inner(self) -> SimInput {
        self.input
    }
}

impl PostSimCtx {
    #[inline]
    pub fn output(&self) -> &SimOutput {
        &self.output
    }
}
