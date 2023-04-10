//! Via layout generators.
//!
//! These generators make it easy to implement common
//! via and via array configurations.
//!
//! You can always implement your own generators if none of the ones here
//! suit your needs. If your generator is sufficiently general-purpose,
//! please consider contributing it to Substrate.

use subgeom::bbox::{Bbox, BoundBox};
use subgeom::transform::TranslateOwned;
use subgeom::{Dims, Dir, ExpandMode, Rect};

use super::ViaExpansion;
use crate::layout::cell::Element;
use crate::layout::group::elements::ElementGroup;
use crate::layout::layers::{LayerBoundBox, LayerKey, LayerSpec};
use crate::layout::placement::align::AlignRect;

/// An enumeration of the layer/metal locations in a via stack.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Ord, PartialOrd)]
enum MetalZ {
    /// The bottom layer.
    Bot,
    /// The top layer.
    Top,
}

/// Data associated with each metal location in a via stack.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
struct MetalInfo<T> {
    /// An array containing data of type `T`.
    ///
    /// The first item in the array corresponds to [`MetalZ::Bot`];
    /// the second item corresponds to [`MetalZ::Top`].
    /// This ordering is subject to change and should not be relied upon.
    ///
    /// These indices are specified by the [`MetalInfo::index`] function.
    data: [T; 2],
}

impl<T> MetalInfo<T> {
    /// Creates a new [`MetalInfo`].
    pub fn new(bot: T, top: T) -> Self {
        Self { data: [bot, top] }
    }

    /// Converts a [`MetalZ`] to an index into the [data array](MetalInfo::data).
    fn index(z: MetalZ) -> usize {
        match z {
            MetalZ::Bot => 0,
            MetalZ::Top => 1,
        }
    }

    /// Creates a new [`MetalInfoBuilder`].
    fn builder() -> MetalInfoBuilder<T> {
        MetalInfoBuilder::<T>::new()
    }
}

impl<T> std::ops::Index<MetalZ> for MetalInfo<T> {
    type Output = T;
    /// Gets the data associated with the given [`MetalZ`].
    ///
    /// Internally, uses the [`MetalInfo::index`] function.
    fn index(&self, index: MetalZ) -> &Self::Output {
        &self.data[Self::index(index)]
    }
}

impl<T> std::ops::IndexMut<MetalZ> for MetalInfo<T> {
    /// Mutably gets the data associated with the given [`MetalZ`].
    ///
    /// Internally, uses the [`MetalInfo::index`] function.
    fn index_mut(&mut self, index: MetalZ) -> &mut Self::Output {
        &mut self.data[Self::index(index)]
    }
}

/// A builder for creating [`MetalInfo`]s.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct MetalInfoBuilder<T> {
    /// The internal data array.
    ///
    /// Behaves analogously to [`MetalInfo::data`].
    data: [Option<T>; 2],
}

impl<T> Default for MetalInfoBuilder<T> {
    /// The default, empty builder.
    fn default() -> Self {
        Self { data: [None, None] }
    }
}

impl<T> MetalInfoBuilder<T> {
    /// Creates a new [`MetalInfoBuilder`].
    pub fn new() -> Self {
        Self { data: [None, None] }
    }

    /// Sets the value of the metal information for a specific location in the via stack.
    pub fn set(&mut self, key: MetalZ, value: T) -> &mut Self {
        self.data[MetalInfo::<T>::index(key)] = Some(value);
        self
    }
}

impl<T> MetalInfoBuilder<T>
where
    T: Clone,
{
    /// Creates a [`MetalInfo`] from this builder by cloning and unwrapping the data.
    ///
    /// # Panics
    ///
    /// This function will panic if data for one or more possible
    /// values of [`MetalZ`] is not specified.
    pub fn build(&self) -> MetalInfo<T> {
        MetalInfo {
            data: [
                self.data[0].as_ref().unwrap().clone(),
                self.data[1].as_ref().unwrap().clone(),
            ],
        }
    }
}

impl<T> MetalInfo<T>
where
    T: BoundBox,
{
    /// Computes the union of the bounding boxes of the top and bottom elements.
    #[allow(dead_code)]
    pub(crate) fn union(&self) -> Bbox {
        self[MetalZ::Bot].bbox().union(self[MetalZ::Top].bbox())
    }
    /// Computes the (possibly empty) intersection of the bounding boxes
    /// of the top and bottom elements.
    pub(crate) fn intersection(&self) -> Bbox {
        self[MetalZ::Bot]
            .bbox()
            .intersection(self[MetalZ::Top].bbox())
    }
}

/// Values that come in pairs – one for the bottom metal, one for the top metal.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Pair<T> {
    bot: T,
    top: T,
}

impl<T> From<MetalInfo<T>> for Pair<T>
where
    T: Copy,
{
    fn from(value: MetalInfo<T>) -> Self {
        Self {
            bot: value[MetalZ::Bot],
            top: value[MetalZ::Top],
        }
    }
}

impl<T> From<Pair<T>> for MetalInfo<T> {
    fn from(value: Pair<T>) -> Self {
        Self::new(value.bot, value.top)
    }
}

/// Dimensions relevant to creating a tiled via array.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct ViaArrayDims {
    /// Extension of the metal layer beyond the edge of the via array.
    extension: MetalInfo<Dims>,
    /// Determines whether the extension of each metal layer is fixed.
    fixed: MetalInfo<bool>,
    /// Dimensions of each drawn via.
    via_size: Dims,
    /// Spacing between vias.
    via_spacing: Dims,
}

impl ViaArrayDims {
    /// Creates a new [`ViaArrayDims`] struct storing the given dimensions.
    pub fn new(
        bot_extension: Dims,
        bot_fixed: bool,
        top_extension: Dims,
        top_fixed: bool,
        via_size: Dims,
        via_spacing: Dims,
    ) -> Self {
        Self {
            extension: MetalInfo::new(bot_extension, top_extension),
            fixed: MetalInfo::new(bot_fixed, top_fixed),
            via_size,
            via_spacing,
        }
    }

    /// Transposes the extension dimensions on the given layer.
    ///
    /// See [`Dims::transpose`] for a description of what
    /// transposing does.
    fn transpose_extension(mut self, z: MetalZ) -> Self {
        self.extension[z] = self.extension[z].transpose();
        self
    }

    /// Returns the sizes on each layer of the tiled via array.
    ///
    /// The via array will be composed of `nx` vias in the
    /// x-direction, and `ny` vias in the y-direction.
    #[inline]
    pub fn size(&self, nx: usize, ny: usize) -> Pair<Rect> {
        self._size(nx, ny).into()
    }

    /// A helper function for sizing the via array.
    ///
    /// Returns the physical dimensions of this array on the top and bottom layers.
    ///
    /// See [`ViaArrayDims::size`] for the public-facing sizing API.
    fn _size(&self, nx: usize, ny: usize) -> MetalInfo<Rect> {
        debug_assert!(nx >= 1);
        debug_assert!(ny >= 1);

        let array = self.array_dims(nx, ny).into_rect();

        let mut builder = MetalInfo::<Rect>::builder();

        for z in [MetalZ::Bot, MetalZ::Top] {
            let rect = array.expand_dims(self.extension[z], ExpandMode::All);
            builder.set(z, rect);
        }

        builder.build()
    }

    /// Calculates the size of the via layer bounding box.
    ///
    /// Does not consider extension on the top/bottom metal layers.
    fn array_dims(&self, nx: usize, ny: usize) -> Dims {
        debug_assert!(nx >= 1);
        debug_assert!(ny >= 1);

        self.via_size * (nx, ny) + self.via_spacing * (nx - 1, ny - 1)
    }

    /// Calculates the maximum number of vias that can be tiled along the given direction
    /// on the given metal layer.
    fn max_n_metal(&self, dir: Dir, z: MetalZ, metal: Rect) -> usize {
        let line_and_space = self.via_size.dim(dir) + self.via_spacing.dim(dir);

        // The max number of contacts that can be placed within the metal bounding box.
        let max = (metal.length(dir) + self.via_spacing.dim(dir) - 2 * self.extension[z].dim(dir))
            / line_and_space;

        if max >= 1 {
            // `i64::MAX` may be larger than `usize::MAX` on some (probably non-standard) platforms.
            // So perform the conversion carefully.
            usize::try_from(max).unwrap_or(usize::MAX)
        } else {
            0
        }
    }

    /// The max number of vias that can fit within
    /// the overlap region in the given direction.
    fn max_n_ov(&self, dir: Dir, ov: Rect) -> usize {
        let line_and_space = self.via_size.dim(dir) + self.via_spacing.dim(dir);

        // The max number of contacts that can be placed in the overlap region.
        let max = (ov.length(dir) + self.via_spacing.dim(dir)) / line_and_space;

        if max >= 1 {
            usize::try_from(max).unwrap_or(usize::MAX)
        } else {
            0
        }
    }

    /// The max number of via contacts in the given direction.
    fn max_n(&self, dir: Dir, metals: MetalInfo<Rect>, ov: Rect) -> usize {
        let mut n = usize::MAX;

        for z in [MetalZ::Bot, MetalZ::Top] {
            n = std::cmp::min(n, self.max_n_metal(dir, z, metals[z]));
        }
        n = std::cmp::min(n, self.max_n_ov(dir, ov));

        n
    }

    /// The max number of via contacts in both directions.
    fn max_ns(&self, metals: MetalInfo<Rect>) -> (usize, usize) {
        let ov = metals.intersection();
        if ov.is_empty() {
            // If the metals don't overlap, cannot place any contacts.
            return (0, 0);
        }

        let ov = ov.into_rect();

        let nx = self.max_n(Dir::Horiz, metals, ov);
        let ny = self.max_n(Dir::Vert, metals, ov);

        (nx, ny)
    }

    /// Selects the maximum possible number of vias given layer geometry and expansion mode.
    fn select_ns(&self, metals: MetalInfo<Rect>, expansion: ViaExpansion) -> (usize, usize) {
        let (nx, ny) = self.max_ns(metals);
        if nx == 0 || ny == 0 {
            use std::cmp::max;
            match expansion {
                ViaExpansion::None => (0, 0),
                ViaExpansion::Minimum => (1, 1),
                ViaExpansion::LongerDirection => (max(nx, 1), max(ny, 1)),
            }
        } else {
            (nx, ny)
        }
    }
}

/// A via array that draws the maximum possible number of vias,
/// possibly transposing extensions.
pub struct MaxViaArray {
    /// The dimensions relevant to sizing the via array.
    dims: ViaArrayDims,

    /// The [`LayerKey`]s of the layers involved.
    metal_layers: MetalInfo<LayerKey>,

    /// The via layer.
    via_layer: LayerKey,

    /// The expansion mode.
    ///
    /// See [`ViaExpansion`] for more information.
    expansion: ViaExpansion,

    /// Layout database grid size as determined by the PDK.
    grid: i64,
}

impl MaxViaArray {
    /// Creates a new [`MaxViaArray`] with the given parameters.
    pub fn new(
        dims: ViaArrayDims,
        bot_metal: LayerKey,
        top_metal: LayerKey,
        via_layer: LayerKey,
        expansion: ViaExpansion,
        grid: i64,
    ) -> Self {
        Self {
            dims,
            metal_layers: MetalInfo::new(bot_metal, top_metal),
            via_layer,
            expansion,
            grid,
        }
    }

    /// Draws the via array, returning an [`ElementGroup`].
    pub fn draw(&self, bot: Rect, top: Rect) -> ElementGroup {
        self._draw(MetalInfo::new(bot, top))
    }

    /// A helper function for [`MaxViaArray::draw`].
    fn _draw(&self, metals: MetalInfo<Rect>) -> ElementGroup {
        let mut max = 0;
        let mut min_diff = i64::MAX;
        let mut group = None;

        for tt in [false, true] {
            for tb in [false, true] {
                let mut dims = self.dims;
                if (dims.fixed[MetalZ::Top] && tt) || (dims.fixed[MetalZ::Bot] && tb) {
                    continue;
                }
                if tt {
                    dims = dims.transpose_extension(MetalZ::Top);
                }
                if tb {
                    dims = dims.transpose_extension(MetalZ::Bot);
                }

                let (nx, ny) = dims.select_ns(metals, self.expansion);
                if nx * ny >= max {
                    let mut tmp_group = FixedSizeViaArray {
                        dims,
                        nx,
                        ny,
                        metal_layers: self.metal_layers,
                        via_layer: self.via_layer,
                        grid: self.grid,
                    }
                    .draw();
                    let ov = metals.intersection();

                    tmp_group.align_centers_gridded(ov, self.grid);

                    let mut diff = 0;
                    for z in [MetalZ::Bot, MetalZ::Top] {
                        let metal_bbox = tmp_group.layer_bbox(self.metal_layers[z]);
                        diff += metal_bbox.into_rect().area()
                            - metal_bbox.intersection(metals[z].bbox()).into_rect().area();
                    }

                    if nx * ny > max || diff <= min_diff {
                        min_diff = diff;
                        group = Some(tmp_group);
                    }
                    max = nx * ny;
                }
            }
        }

        group.unwrap()
    }
}

/// A via array that draws the maximum possible number of vias
/// while keeping extensions fixed.
pub struct MaxFixedExtensionViaArray {
    /// The dimensions relevant to sizing the via array.
    dims: ViaArrayDims,

    /// The [`LayerKey`]s of the layers involved.
    metal_layers: MetalInfo<LayerKey>,

    /// The via layer.
    via_layer: LayerKey,

    /// The expansion mode.
    ///
    /// See [`ViaExpansion`] for more information.
    expansion: ViaExpansion,

    /// Layout database grid size as determined by the PDK.
    grid: i64,
}

impl MaxFixedExtensionViaArray {
    /// Draws the via array, returning an [`ElementGroup`].
    pub fn draw(&self, bot: Rect, top: Rect) -> ElementGroup {
        self._draw(MetalInfo::new(bot, top))
    }

    /// A helper function for [`MaxViaArray::draw`].
    fn _draw(&self, metals: MetalInfo<Rect>) -> ElementGroup {
        let (nx, ny) = self.dims.select_ns(metals, self.expansion);

        let mut group = FixedSizeViaArray {
            dims: self.dims,
            nx,
            ny,
            metal_layers: self.metal_layers,
            via_layer: self.via_layer,
            grid: self.grid,
        }
        .draw();

        let ov = metals.intersection();

        group.align_centers_gridded(ov, self.grid);
        group
    }
}

/// A via array with fixed orientations on each layer.
///
/// The [`draw`](FixedSizeViaArray::draw) method draws the via
/// array and centers it at `(0, 0)`.
pub struct FixedSizeViaArray {
    dims: ViaArrayDims,

    /// Width of the via array (in number of vias).
    nx: usize,
    /// Height of the via array (in number of vias).
    ny: usize,

    /// The [`LayerKey`]s of the layers involved.
    metal_layers: MetalInfo<LayerKey>,
    /// The via layer.
    via_layer: LayerKey,

    /// Layout database grid size as determined by the PDK.
    grid: i64,
}

impl FixedSizeViaArray {
    /// Draws the via array, returning an [`ElementGroup`].
    pub fn draw(&self) -> ElementGroup {
        let nx = self.nx;
        let ny = self.ny;

        assert!(nx >= 1);
        assert!(ny >= 1);

        let mut group = ElementGroup::new();

        let array = self.dims.array_dims(nx, ny).into_rect();

        for z in [MetalZ::Bot, MetalZ::Top] {
            let rect = array.expand_dims(self.dims.extension[z], ExpandMode::All);
            group.add(Element::new(LayerSpec::drawing(self.metal_layers[z]), rect));
        }

        for i in 0..nx {
            for j in 0..ny {
                let base = ((self.dims.via_size + self.dims.via_spacing) * (i, j)).into_point();
                let via = Rect::with_dims(self.dims.via_size).translate_owned(base);
                group.add(Element::new(LayerSpec::drawing(self.via_layer), via));
            }
        }

        group.align_centers_gridded(Bbox::zero(), self.grid);

        group
    }
}
