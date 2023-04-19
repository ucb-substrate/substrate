use std::collections::HashMap;
use std::path::PathBuf;

use derive_builder::Builder;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use self::waveform::{binary_search_before, SharedWaveform};
use crate::error::Result;
use crate::schematic::signal::NamedSignalPathBuf;
use crate::units::SiValue;

pub mod bits;
pub mod context;
pub mod testbench;
pub mod waveform;

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimInput {
    pub work_dir: PathBuf,
    pub opts: SimOpts,
    pub includes: Vec<PathBuf>,
    pub libs: Vec<Lib>,
    pub save: Save,
    /// Initial conditions for transient analysis.
    pub ic: HashMap<String, SiValue>,
    pub measurements: Vec<Measurement>,
    pub analyses: Vec<Analysis>,
    pub output_format: OutputFormat,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Use any format that can be read into Substrate data.
    #[default]
    DefaultReadable,
    /// Use any format that can be viewed by a human.
    ///
    /// If you want to ensure data can be programatically read into Substrate,
    /// use [`OutputFormat::DefaultReadable`] instead. This format
    /// is only intended for human viewability, not programatic readability.
    DefaultViewable,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimOutput {
    pub data: Vec<AnalysisData>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimOpts {
    /// Simulation temperature, in degrees Celsius.
    pub temp: Option<f64>,
    /// The temperature at which model parameters were measured, in degrees Celsius.
    pub tnom: Option<f64>,
    pub gmin: Option<f64>,
    pub iabstol: Option<f64>,
    pub reltol: Option<f64>,
    pub bashrc: Option<PathBuf>,
    /// Flags to pass to the simulator invocation.
    pub flags: Option<String>,
    pub other: HashMap<String, String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Lib {
    pub path: PathBuf,
    pub section: String,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub enum Save {
    #[default]
    All,
    None,
    Signals(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct Measurement {
    analysis_mode: String,
    name: String,
    expr: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Analysis {
    Op(OpAnalysis),
    Dc(DcAnalysis),
    Tran(TranAnalysis),
    Ac(AcAnalysis),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AnalysisType {
    Op,
    Dc,
    Tran,
    Ac,
    Other,
}

#[derive(Debug, Default, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct OpAnalysis {}

impl OpAnalysis {
    #[inline]
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpData {
    /// All saved signals.
    pub data: HashMap<String, ScalarSignal>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranData {
    /// All saved signals, not including time.
    pub data: HashMap<String, RealSignal>,
    pub time: RealSignal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcData {
    /// All saved signals, not including frequency.
    pub data: HashMap<String, ComplexSignal>,
    pub freq: RealSignal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DcData {
    /// All saved signals.
    pub data: HashMap<String, RealSignal>,
}

#[derive(Debug, Clone, Builder, PartialEq, Serialize, Deserialize)]
pub struct DcAnalysis {
    /// The name of the source or parameter to sweep.
    #[builder(setter(into))]
    pub sweep: String,
    pub start: f64,
    pub stop: f64,
    pub step: f64,
}

impl DcAnalysis {
    #[inline]
    pub fn builder() -> DcAnalysisBuilder {
        DcAnalysisBuilder::default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Builder)]
pub struct TranAnalysis {
    pub stop: f64,
    pub step: f64,
    #[builder(default)]
    pub start: f64,
    #[builder(default, setter(strip_option))]
    pub strobe_period: Option<f64>,
}

impl TranAnalysis {
    #[inline]
    pub fn builder() -> TranAnalysisBuilder {
        TranAnalysisBuilder::default()
    }
}

#[derive(Debug, Clone, Builder, PartialEq, Serialize, Deserialize)]
pub struct AcAnalysis {
    pub fstart: f64,
    pub fstop: f64,
    pub points: usize,
    pub sweep: SweepMode,
}

impl AcAnalysis {
    #[inline]
    pub fn builder() -> AcAnalysisBuilder {
        AcAnalysisBuilder::default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScalarSignal {
    pub value: f64,
    pub quantity: Quantity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealSignal {
    pub values: Vec<f64>,
    pub quantity: Quantity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplexSignal {
    pub real: Vec<f64>,
    pub imag: Vec<f64>,
    pub quantity: Quantity,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum Quantity {
    Voltage,
    Current,
    Frequency,
    Time,
    Temperature,
    Unknown,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum SweepMode {
    Dec,
    Oct,
    Lin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnalysisData {
    Op(OpData),
    Tran(TranData),
    Ac(AcData),
    Dc(DcData),
    Other,
}

impl AnalysisData {
    pub fn analysis_type(&self) -> AnalysisType {
        match self {
            Self::Op(_) => AnalysisType::Op,
            Self::Tran(_) => AnalysisType::Tran,
            Self::Ac(_) => AnalysisType::Ac,
            Self::Dc(_) => AnalysisType::Dc,
            Self::Other => AnalysisType::Other,
        }
    }

    /// Get the results of an operating point analysis.
    ///
    /// # Panics
    ///
    /// This function panics if this analysis does not correspond to an operating point analysis.
    pub fn op(&self) -> &OpData {
        match self {
            Self::Op(x) => x,
            _ => panic!("Expected op analysis, got {:?}", self.analysis_type()),
        }
    }

    /// Get the results of a transient analysis.
    ///
    /// # Panics
    ///
    /// This function panics if this analysis does not correspond to a transient analysis.
    pub fn tran(&self) -> &TranData {
        match self {
            Self::Tran(x) => x,
            _ => panic!("Expected tran analysis, got {:?}", self.analysis_type()),
        }
    }

    /// Get the results of an AC analysis.
    ///
    /// # Panics
    ///
    /// This function panics if this analysis does not correspond to an AC analysis.
    pub fn ac(&self) -> &AcData {
        match self {
            Self::Ac(x) => x,
            _ => panic!("Expected ac analysis, got {:?}", self.analysis_type()),
        }
    }

    /// Get the results of a DC analysis.
    ///
    /// # Panics
    ///
    /// This function panics if this analysis does not correspond to a DC analysis.
    pub fn dc(&self) -> &DcData {
        match self {
            Self::Dc(x) => x,
            _ => panic!("Expected dc analysis, got {:?}", self.analysis_type()),
        }
    }
}

impl From<OpData> for AnalysisData {
    fn from(value: OpData) -> Self {
        Self::Op(value)
    }
}
impl From<TranData> for AnalysisData {
    fn from(value: TranData) -> Self {
        Self::Tran(value)
    }
}
impl From<AcData> for AnalysisData {
    fn from(value: AcData) -> Self {
        Self::Ac(value)
    }
}
impl From<DcData> for AnalysisData {
    fn from(value: DcData) -> Self {
        Self::Dc(value)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulatorOpts {
    pub opts: HashMap<String, String>,
}

pub trait Simulator {
    fn new(opts: SimulatorOpts) -> Result<Self>
    where
        Self: Sized;
    fn simulate(&self, input: SimInput) -> Result<SimOutput>;
    fn node_voltage_string(&self, path: &NamedSignalPathBuf) -> String;
}

impl Analysis {
    pub fn analysis_type(&self) -> AnalysisType {
        match self {
            Analysis::Op(_) => AnalysisType::Op,
            Analysis::Tran(_) => AnalysisType::Tran,
            Analysis::Ac(_) => AnalysisType::Ac,
            Analysis::Dc(_) => AnalysisType::Dc,
        }
    }
}

impl From<TranAnalysis> for Analysis {
    fn from(value: TranAnalysis) -> Self {
        Self::Tran(value)
    }
}

impl From<OpAnalysis> for Analysis {
    fn from(value: OpAnalysis) -> Self {
        Self::Op(value)
    }
}

impl From<DcAnalysis> for Analysis {
    fn from(value: DcAnalysis) -> Self {
        Self::Dc(value)
    }
}

impl From<AcAnalysis> for Analysis {
    fn from(value: AcAnalysis) -> Self {
        Self::Ac(value)
    }
}

impl RealSignal {
    #[inline]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns the index of the first value that is at least `v`.
    pub fn where_at_least(&self, v: f64) -> Option<usize> {
        self.values
            .iter()
            .find_position(|&x| *x >= v)
            .map(|(idx, _)| idx)
    }

    pub fn get(&self, idx: usize) -> Option<f64> {
        self.values.get(idx).copied()
    }

    /// Gets the index into the signal
    /// corresponding to the latest value less than or equal to `x`.
    ///
    /// The signal must be monotonically increasing. This is intended
    /// for use with timestamps, eg. from a transient analysis.
    pub fn idx_before_sorted(&self, x: f64) -> Option<usize> {
        binary_search_before(&self.values, x)
    }
}

impl std::ops::Index<usize> for RealSignal {
    type Output = f64;
    fn index(&self, index: usize) -> &Self::Output {
        self.values.index(index)
    }
}

impl std::ops::IndexMut<usize> for RealSignal {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.values.index_mut(index)
    }
}

impl TranData {
    pub fn signal(&self, name: &str) -> Option<&RealSignal> {
        self.data.get(name)
    }

    pub fn waveform(&self, name: &str) -> Option<SharedWaveform> {
        let x = self.data.get(name)?;
        Some(SharedWaveform::from_signal(&self.time, x))
    }

    pub fn time_waveform(&self) -> SharedWaveform<'_> {
        SharedWaveform::from_signal(&self.time, &self.time)
    }
}
