use std::path::PathBuf;

use arcstr::ArcStr;
use calibre::drc::{run_drc, DrcParams};
use calibre::lvs::{run_lvs, LvsParams, LvsStatus};
use calibre::pex::{run_pex, PexParams};
use derive_builder::Builder;
use substrate::error::ErrorSource;
use substrate::verification::drc::{DrcError, DrcOutput, DrcSummary, DrcTool};
use substrate::verification::lvs::{LvsOutput, LvsSummary, LvsTool};
use substrate::verification::pex::{PexOutput, PexSummary, PexTool};

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
#[non_exhaustive]
#[builder(pattern = "owned")]
pub struct CalibreDrc {
    pub rules_file: PathBuf,
    #[builder(setter(strip_option), default)]
    pub runset_file: Option<PathBuf>,
}

impl CalibreDrc {
    pub fn builder() -> CalibreDrcBuilder {
        CalibreDrcBuilder::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
#[non_exhaustive]
pub struct CalibreLvs {
    rules_file: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
#[non_exhaustive]
pub struct CalibrePex {
    rules_file: PathBuf,
}

impl DrcTool for CalibreDrc {
    fn run_drc(
        &self,
        input: substrate::verification::drc::DrcInput,
    ) -> substrate::error::Result<substrate::verification::drc::DrcOutput> {
        let output = run_drc(&DrcParams {
            work_dir: &input.work_dir,
            layout_path: &input.layout_path,
            cell_name: &input.cell_name,
            rules_path: &self.rules_file,
            runset_path: self.runset_file.as_deref(),
        })
        .map_err(|e| ErrorSource::Internal(format!("{e}")))?;
        let errors = output
            .rule_checks
            .into_iter()
            .map(|rc| DrcError {
                name: ArcStr::from(rc.name),
                ..Default::default()
            })
            .collect::<Vec<_>>();

        let summary = if errors.is_empty() {
            DrcSummary::Pass
        } else {
            DrcSummary::Fail
        };

        Ok(DrcOutput { summary, errors })
    }
}

impl CalibreLvs {
    pub fn new(rules_file: impl Into<PathBuf>) -> Self {
        Self {
            rules_file: rules_file.into(),
        }
    }
}

impl LvsTool for CalibreLvs {
    fn run_lvs(
        &self,
        input: substrate::verification::lvs::LvsInput,
    ) -> substrate::error::Result<substrate::verification::lvs::LvsOutput> {
        let output = run_lvs(&LvsParams {
            work_dir: &input.work_dir,
            layout_path: &input.layout_path,
            layout_cell_name: &input.layout_cell_name,
            source_paths: &input.source_paths,
            source_cell_name: &input.source_cell_name,
            rules_path: &self.rules_file,
        })
        .map_err(|e| ErrorSource::Internal(format!("{e}")))?;

        let summary = match output.status {
            LvsStatus::Correct => LvsSummary::Pass,
            LvsStatus::Incorrect => LvsSummary::Fail,
        };

        Ok(LvsOutput {
            summary,
            errors: Vec::new(), // TODO: Implement recovering errors from output files.
        })
    }
}

impl CalibrePex {
    pub fn new(rules_file: impl Into<PathBuf>) -> Self {
        Self {
            rules_file: rules_file.into(),
        }
    }
}

impl PexTool for CalibrePex {
    fn run_pex(
        &self,
        input: substrate::verification::pex::PexInput,
    ) -> substrate::error::Result<substrate::verification::pex::PexOutput> {
        let output = run_pex(&PexParams {
            work_dir: &input.work_dir,
            layout_path: &input.layout_path,
            layout_cell_name: &input.layout_cell_name,
            source_paths: &input.source_paths,
            source_cell_name: &input.source_cell_name,
            rules_path: &self.rules_file,
            pex_netlist_path: &input.pex_netlist_path,
            level: calibre::pex::PexLevel::default(),
        })
        .map_err(|e| ErrorSource::Internal(format!("{e}")))?;

        let summary = match output.status {
            LvsStatus::Correct => PexSummary::Pass,
            LvsStatus::Incorrect => PexSummary::Fail,
        };

        Ok(PexOutput {
            summary,
            errors: Vec::new(), // TODO: Implement recovering errors from output files.
        })
    }
}
