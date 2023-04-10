use std::path::PathBuf;

use arcstr::ArcStr;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SlotMap};

use self::error::StdCellError;
use crate::component::{Component, View};

pub mod error;

new_key_type! {
    /// A unique identifier for [standard cells](StdCellData).
    pub struct StdCellKey;

    /// A unique identifier for [standard cell libraries](StdCellLib).
    pub struct StdCellLibKey;
}

#[derive(Debug, Default, Clone)]
pub struct StdCellLibData {
    name: ArcStr,
    cells: SlotMap<StdCellKey, StdCellEntry>,

    layout_source: Option<PathBuf>,
    schematic_source: Option<PathBuf>,
}

pub struct StdCellLibEntry {
    id: StdCellLibKey,
    data: StdCellLibData,
}

#[derive(Debug, Clone, Builder)]
pub struct StdCellData {
    #[builder(setter(into))]
    name: ArcStr,

    #[builder(default, setter(strip_option, into))]
    layout_name: Option<ArcStr>,
    #[builder(default, setter(strip_option, into))]
    layout_source: Option<PathBuf>,

    #[builder(default, setter(strip_option, into))]
    schematic_name: Option<ArcStr>,
    #[builder(default, setter(strip_option, into))]
    schematic_source: Option<PathBuf>,

    function: Function,
    #[builder(default = "1")]
    strength: usize,
}

#[derive(Debug, Clone, Builder)]
pub struct StdCellEntry {
    id: StdCellKey,
    data: StdCellData,
}

#[derive(Debug, Copy, Clone, Builder)]
pub struct StdCellRef<'a> {
    library_id: StdCellLibKey,
    inner: &'a StdCellEntry,
}

impl<'a> std::ops::Deref for StdCellRef<'a> {
    type Target = StdCellEntry;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'a> StdCellRef<'a> {
    #[inline]
    pub fn id(&self) -> StdCellId {
        StdCellId::new(self.library_id, self.inner.cell_id())
    }

    #[inline]
    pub fn library_id(&self) -> StdCellLibKey {
        self.library_id
    }

    #[inline]
    pub fn into_inner(self) -> &'a StdCellEntry {
        self.inner
    }

    #[inline]
    fn new(library_id: StdCellLibKey, inner: &'a StdCellEntry) -> Self {
        Self { library_id, inner }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Function {
    And2,
    And3,
    Nand2,
    Nand3,
    Or2,
    Or3,
    Nor2,
    Nor3,
    Inv,
    Buf,
    Mux2,
    Mux3,
    Mux4,
    Xnor2,
    Xnor3,
    Xnor4,
    Xor2,
    Xor3,
    Xor4,
    Other(String),
}

pub struct StdCellDb {
    libraries: SlotMap<StdCellLibKey, StdCellLibEntry>,
    default_lib: Option<StdCellLibKey>,
}

#[inline]
fn view_unsupported(view: View) -> crate::error::SubstrateError {
    crate::component::error::Error::ViewUnsupported(view).into()
}

impl StdCellLibData {
    pub fn new(name: impl Into<ArcStr>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    #[inline]
    pub fn add_cell(&mut self, data: StdCellData) -> StdCellKey {
        self.cells
            .insert_with_key(move |k| StdCellEntry { id: k, data })
    }

    #[inline]
    pub fn cells(&self) -> impl Iterator<Item = &StdCellEntry> + '_ {
        self.cells.values()
    }

    #[inline]
    pub fn set_layout_source(&mut self, source: impl Into<PathBuf>) {
        self.layout_source = Some(source.into());
    }

    #[inline]
    pub fn set_schematic_source(&mut self, source: impl Into<PathBuf>) {
        self.schematic_source = Some(source.into());
    }

    pub fn try_cell_named(&self, name: &str) -> crate::error::Result<&StdCellEntry> {
        self.cells().find(|c| c.name() == name).ok_or_else(|| {
            StdCellError::CellNameNotFound {
                cell: name.to_string(),
                lib: self.name.to_string(),
            }
            .into()
        })
    }

    pub fn try_cell(&self, id: StdCellKey) -> crate::error::Result<&StdCellEntry> {
        self.cells.get(id).ok_or_else(|| {
            StdCellError::CellIdNotFound {
                cell: id,
                lib: self.name.to_string(),
            }
            .into()
        })
    }

    pub fn source(&self, view: View) -> Option<&PathBuf> {
        match view {
            View::Schematic => self.schematic_source.as_ref(),
            View::Layout => self.layout_source.as_ref(),
            _ => None,
        }
    }

    pub fn try_source(&self, view: View) -> crate::error::Result<&PathBuf> {
        let source = match view {
            View::Schematic => self.schematic_source.as_ref(),
            View::Layout => self.layout_source.as_ref(),
            _ => None,
        };
        source.ok_or_else(|| view_unsupported(view))
    }

    pub fn name(&self) -> &ArcStr {
        &self.name
    }
}

impl StdCellLibEntry {
    #[inline]
    pub fn id(&self) -> StdCellLibKey {
        self.id
    }

    #[inline]
    pub fn cells(&self) -> impl Iterator<Item = StdCellRef> + '_ {
        self.data
            .cells()
            .map(|cell| StdCellRef::new(self.id(), cell))
    }

    pub fn try_cell_named(&self, name: &str) -> crate::error::Result<StdCellRef> {
        self.data
            .try_cell_named(name)
            .map(|cell| StdCellRef::new(self.id(), cell))
    }

    #[inline]
    pub fn try_cell(&self, id: StdCellKey) -> crate::error::Result<StdCellRef> {
        self.data
            .try_cell(id)
            .map(|cell| StdCellRef::new(self.id(), cell))
    }

    #[inline]
    pub fn source(&self, view: View) -> Option<&PathBuf> {
        self.data.source(view)
    }

    #[inline]
    pub fn try_source(&self, view: View) -> crate::error::Result<&PathBuf> {
        self.data.try_source(view)
    }

    #[inline]
    pub fn name(&self) -> &ArcStr {
        self.data.name()
    }
}

impl StdCellData {
    #[inline]
    pub fn name(&self) -> &ArcStr {
        &self.name
    }

    pub fn source(&self, view: View) -> Option<&PathBuf> {
        match view {
            View::Schematic => self.schematic_source.as_ref(),
            View::Layout => self.layout_source.as_ref(),
            _ => None,
        }
    }

    pub fn try_source(&self, view: View) -> crate::error::Result<&PathBuf> {
        let source = match view {
            View::Schematic => self.schematic_source.as_ref(),
            View::Layout => self.layout_source.as_ref(),
            _ => None,
        };
        source.ok_or_else(|| view_unsupported(view))
    }

    pub fn view_name(&self, view: View) -> &ArcStr {
        let name = match view {
            View::Schematic => self.schematic_name.as_ref(),
            View::Layout => self.layout_name.as_ref(),
            _ => None,
        };
        name.unwrap_or(&self.name)
    }

    #[inline]
    pub fn function(&self) -> &Function {
        &self.function
    }

    #[inline]
    pub fn strength(&self) -> usize {
        self.strength
    }

    #[inline]
    pub fn builder() -> StdCellDataBuilder {
        StdCellDataBuilder::default()
    }
}

impl StdCellEntry {
    #[inline]
    pub fn name(&self) -> &ArcStr {
        self.data.name()
    }

    #[inline]
    pub fn cell_id(&self) -> StdCellKey {
        self.id
    }

    #[inline]
    pub fn source(&self, view: View) -> Option<&PathBuf> {
        self.data.source(view)
    }

    #[inline]
    pub fn try_source(&self, view: View) -> crate::error::Result<&PathBuf> {
        self.data.try_source(view)
    }

    #[inline]
    pub fn view_name(&self, view: View) -> &ArcStr {
        self.data.view_name(view)
    }

    #[inline]
    pub fn function(&self) -> &Function {
        self.data.function()
    }

    #[inline]
    pub fn strength(&self) -> usize {
        self.data.strength()
    }
}

impl Default for StdCellDb {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl StdCellDb {
    pub fn new() -> Self {
        Self {
            libraries: SlotMap::with_key(),
            default_lib: None,
        }
    }

    #[inline]
    pub fn add_lib(&mut self, data: StdCellLibData) -> StdCellLibKey {
        self.libraries
            .insert_with_key(move |k| StdCellLibEntry { id: k, data })
    }

    #[inline]
    pub fn set_default_lib(&mut self, id: StdCellLibKey) {
        self.default_lib = Some(id);
    }

    #[inline]
    pub fn default_lib(&self) -> Option<&StdCellLibEntry> {
        self.lib(self.default_lib?)
    }

    #[inline]
    pub fn try_default_lib(&self) -> crate::error::Result<&StdCellLibEntry> {
        self.default_lib()
            .ok_or_else(|| StdCellError::NoDefaultLibrary.into())
    }

    pub fn lib_named(&self, name: &str) -> Option<&StdCellLibEntry> {
        self.libraries.values().find(|l| l.name() == name)
    }

    pub fn try_lib_named(&self, name: &str) -> crate::error::Result<&StdCellLibEntry> {
        self.lib_named(name)
            .ok_or_else(|| StdCellError::LibNameNotFound(name.to_string()).into())
    }

    #[inline]
    pub fn lib(&self, id: StdCellLibKey) -> Option<&StdCellLibEntry> {
        self.libraries.get(id)
    }

    pub fn try_lib(&self, id: StdCellLibKey) -> crate::error::Result<&StdCellLibEntry> {
        self.lib(id)
            .ok_or_else(|| StdCellError::LibIdNotFound(id).into())
    }

    pub fn try_cell(&self, id: StdCellId) -> crate::error::Result<StdCellRef> {
        let lib = self.try_lib(id.lib)?;
        let cell = lib.try_cell(id.cell)?;
        Ok(cell)
    }

    pub fn try_lib_and_cell(
        &self,
        id: StdCellId,
    ) -> crate::error::Result<(&StdCellLibEntry, StdCellRef)> {
        let lib = self.try_lib(id.lib)?;
        let cell = lib.try_cell(id.cell)?;
        Ok((lib, cell))
    }

    pub fn source(&self, id: StdCellId, view: View) -> crate::error::Result<&PathBuf> {
        let (lib, cell) = self.try_lib_and_cell(id)?;

        if let Some(path) = cell.inner.source(view.clone()) {
            Ok(path)
        } else if let Some(path) = lib.source(view) {
            Ok(path)
        } else {
            Err(
                crate::component::error::Error::ViewUnsupported(crate::component::View::Layout)
                    .into(),
            )
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Serialize, Deserialize)]
pub struct StdCellId {
    lib: StdCellLibKey,
    cell: StdCellKey,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct StdCell {
    params: StdCellId,
    name: ArcStr,
}

impl StdCellId {
    pub fn new(lib: StdCellLibKey, cell: StdCellKey) -> Self {
        Self { lib, cell }
    }
}

impl Component for StdCell {
    type Params = StdCellId;
    fn new(params: &Self::Params, ctx: &crate::data::SubstrateCtx) -> crate::error::Result<Self> {
        let db = ctx.std_cell_db();
        let cell = db.try_cell(*params)?;
        let name = cell.name();
        Ok(Self {
            params: *params,
            name: name.clone(),
        })
    }

    fn name(&self) -> ArcStr {
        self.name.clone()
    }

    fn schematic(
        &self,
        ctx: &mut crate::schematic::context::SchematicCtx,
    ) -> crate::error::Result<()> {
        let db = ctx.inner().std_cell_db();
        let cell = db.try_cell(self.params)?;
        let source = db.source(self.params, View::Schematic)?;
        ctx.import_spice(cell.view_name(View::Schematic), source)?;
        Ok(())
    }

    fn layout(&self, ctx: &mut crate::layout::context::LayoutCtx) -> crate::error::Result<()> {
        let db = ctx.inner().std_cell_db();
        let cell = db.try_cell(self.params)?;
        let view = View::Layout;
        let source = db.source(self.params, view.clone())?;
        ctx.from_gds_flattened(source, cell.view_name(view))?;
        Ok(())
    }
}
