//! Routing jogs.

use derive_builder::Builder;
use subgeom::{Dir, Edge, Rect, Sign, Span};

use crate::layout::cell::Element;
use crate::layout::group::Group;
use crate::layout::layers::{LayerPurpose, UserLayer};
use crate::layout::{Draw, DrawRef};

/// A collection of several [`SJog`]s between the provided source and destination spans.
#[derive(Builder)]
pub struct SimpleJog<const N: usize> {
    dir: Dir,
    src_pos: i64,
    #[builder(setter(into))]
    src: [Span; N],
    #[builder(setter(into))]
    dst: [Span; N],
    line: i64,
    space: i64,
    #[builder(setter(into))]
    layer: UserLayer,
}

impl<const N: usize> SimpleJog<N> {
    #[inline]
    pub fn builder() -> SimpleJogBuilder<N> {
        SimpleJogBuilder::default()
    }
    pub fn generate(&self) -> Group {
        let spec = self.layer.clone().to_spec(LayerPurpose::Drawing);

        let mut group = Group::new();
        let dir = self.dir;
        for i in 0..N {
            let src = self.src[i];
            let dst = self.dst[i];

            let rect = Rect::span_builder()
                .with(
                    dir,
                    Span::new(self.src_pos, self.src_pos + self.space + self.line),
                )
                .with(!dir, src)
                .build();
            group.add(Element::new(spec.clone(), rect));

            let dst_pos = self.dst_pos();

            let rect = Rect::span_builder()
                .with(dir, Span::new(dst_pos - self.space - self.line, dst_pos))
                .with(!dir, dst)
                .build();
            group.add(Element::new(spec.clone(), rect));

            let rect = Rect::span_builder()
                .with(
                    dir,
                    Span::new(
                        self.src_pos + self.space,
                        self.src_pos + self.space + self.line,
                    ),
                )
                .with(!dir, Span::new(src.start(), dst.stop()))
                .build();
            group.add(Element::new(spec.clone(), rect));
        }

        group
    }

    #[inline]
    pub fn dst_pos(&self) -> i64 {
        self.src_pos + self.space + self.line + self.space
    }
}

impl<const N: usize> Draw for SimpleJog<N> {
    fn draw(self) -> crate::error::Result<crate::layout::group::Group> {
        Ok(self.generate())
    }
}

impl<const N: usize> DrawRef for SimpleJog<N> {
    fn draw_ref(&self) -> crate::error::Result<Group> {
        Ok(self.generate())
    }
}

/// A right angle jog from a starting edge to a certain 2D coordinate.
#[derive(Builder, Debug)]
pub struct ElbowJog {
    /// The source edge.
    ///
    /// Determines the initial width and direction of the jog.
    src: Edge,
    /// The end coordinate in the direction of the first leg of the jog.
    coord1: i64,
    /// The end coordinate in the direction of the second leg of the jog.
    coord2: i64,
    /// Width of the second leg of the jog.
    ///
    /// If not specified, both legs will have the same width.
    #[builder(setter(strip_option), default)]
    width2: Option<i64>,
    #[builder(setter(into))]
    /// The layer of the jog.
    layer: UserLayer,
}

impl ElbowJog {
    pub fn builder() -> ElbowJogBuilder {
        ElbowJogBuilder::default()
    }

    pub fn r1(&self) -> Rect {
        Rect::span_builder()
            .with(
                self.src.norm_dir(),
                Span::from_point(self.src.coord()).add_point(self.coord1),
            )
            .with(self.src.edge_dir(), self.src.span())
            .build()
    }

    pub fn r2(&self) -> Rect {
        Rect::span_builder()
            .with(self.src.edge_dir(), self.src.span().add_point(self.coord2))
            .with(
                self.src.norm_dir(),
                Span::with_point_and_length(
                    self.src.side().sign(),
                    self.coord1,
                    if let Some(width2) = self.width2 {
                        width2
                    } else {
                        self.src.span().length()
                    },
                ),
            )
            .build()
    }

    pub fn generate(&self) -> Group {
        let mut group = Group::new();

        let spec = self.layer.clone().to_spec(LayerPurpose::Drawing);

        group.add_element(Element::new(spec.clone(), self.r1()));
        group.add_element(Element::new(spec, self.r2()));

        group
    }
}

impl Draw for ElbowJog {
    fn draw(self) -> crate::error::Result<crate::layout::group::Group> {
        Ok(self.generate())
    }
}

impl DrawRef for ElbowJog {
    fn draw_ref(&self) -> crate::error::Result<Group> {
        Ok(self.generate())
    }
}

#[derive(Builder, Debug)]
pub struct OffsetJog {
    /// The initial direction of the elbow.
    dir: Dir,
    /// The initial sign of the elbow direction.
    ///
    /// Positive means up or right; negative means down or left.
    sign: Sign,
    #[builder(setter(into))]
    src: Rect,
    dst: i64,
    #[builder(setter(into))]
    layer: UserLayer,

    #[builder(default, setter(strip_option))]
    space: Option<i64>,
}

impl OffsetJog {
    pub fn builder() -> OffsetJogBuilder {
        OffsetJogBuilder::default()
    }

    fn line(&self) -> i64 {
        self.src.span(!self.dir).length()
    }

    fn space(&self) -> i64 {
        self.space.unwrap_or_else(|| self.line())
    }

    fn p1(&self) -> i64 {
        self.src.span(self.dir).point(self.sign) + self.sign.as_int() * (self.line() + self.space())
    }

    fn p2(&self) -> i64 {
        self.src.span(self.dir).point(self.sign) + self.sign.as_int() * self.space()
    }

    pub fn r1(&self) -> Rect {
        Rect::span_builder()
            .with(!self.dir, self.src.span(!self.dir))
            .with(self.dir, self.src.span(self.dir).add_point(self.p1()))
            .build()
    }

    pub fn r2(&self) -> Rect {
        Rect::span_builder()
            .with(self.dir, Span::new(self.p1(), self.p2()))
            .with(!self.dir, self.src.span(!self.dir).add_point(self.dst))
            .build()
    }

    pub fn generate(&self) -> Group {
        let mut group = Group::new();

        let spec = self.layer.clone().to_spec(LayerPurpose::Drawing);

        group.add_element(Element::new(spec.clone(), self.r1()));
        group.add_element(Element::new(spec, self.r2()));

        group
    }
}

impl Draw for OffsetJog {
    fn draw(self) -> crate::error::Result<crate::layout::group::Group> {
        Ok(self.generate())
    }
}

impl DrawRef for OffsetJog {
    fn draw_ref(&self) -> crate::error::Result<Group> {
        Ok(self.generate())
    }
}

#[derive(Builder, Debug)]
pub struct SJog {
    src: Rect,
    dst: Rect,
    dir: Dir,
    #[builder(setter(into))]
    layer: UserLayer,
    #[builder(default, setter(strip_option))]
    width: Option<i64>,
    #[builder(default, setter(strip_option))]
    l1: Option<i64>,
    #[builder(default, setter(strip_option))]
    grid: Option<i64>,
}

impl SJog {
    pub fn builder() -> SJogBuilder {
        SJogBuilder::default()
    }

    fn width(&self) -> i64 {
        self.width
            .unwrap_or_else(|| self.src.span(!self.dir).length())
    }

    fn cspan(&self) -> Span {
        let s1 = self.src.span(self.dir);
        let s2 = self.dst.span(self.dir);

        let c = if let Some(l1) = self.l1 {
            if s1.start() > s2.start() {
                s1.start() - l1
            } else {
                s1.stop() + l1
            }
        } else if s1.start() > s2.start() {
            (s1.start() + s2.stop()) / 2
        } else {
            (s2.start() + s1.stop()) / 2
        };

        if let Some(grid) = self.grid {
            Span::from_center_span_gridded(c, self.width(), grid)
        } else {
            Span::from_center_span(c, self.width())
        }
    }

    pub fn r1(&self) -> Rect {
        Rect::span_builder()
            .with(!self.dir, self.src.span(!self.dir))
            .with(self.dir, self.src.span(self.dir).union(self.cspan()))
            .build()
    }

    pub fn r2(&self) -> Rect {
        Rect::span_builder()
            .with(!self.dir, self.xspan())
            .with(self.dir, self.cspan())
            .build()
    }

    pub fn r3(&self) -> Rect {
        Rect::span_builder()
            .with(!self.dir, self.dst.span(!self.dir))
            .with(self.dir, self.dst.span(self.dir).union(self.cspan()))
            .build()
    }

    fn xspan(&self) -> Span {
        self.src.span(!self.dir).union(self.dst.span(!self.dir))
    }

    pub fn generate(&self) -> Group {
        let mut group = Group::new();

        let spec = self.layer.clone().to_spec(LayerPurpose::Drawing);
        for r in [self.r1(), self.r2(), self.r3()] {
            group.add_element(Element::new(spec.clone(), r));
        }

        group
    }
}

impl Draw for SJog {
    fn draw(self) -> crate::error::Result<crate::layout::group::Group> {
        Ok(self.generate())
    }
}

impl DrawRef for SJog {
    fn draw_ref(&self) -> crate::error::Result<Group> {
        Ok(self.generate())
    }
}
