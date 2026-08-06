use std::collections::BTreeMap;

use super::style::{Alignment, Color, ParagraphStyle, TabLeader, TextStyle};

/// Header or footer content for flow pages.
#[derive(Debug, Clone)]
pub struct HeaderFooter {
    pub paragraphs: Vec<HeaderFooterParagraph>,
    /// Distance in points from the page edge, as specified by the section page margins.
    pub distance_from_edge: Option<f64>,
}

/// A paragraph within a header or footer.
#[derive(Debug, Clone)]
pub struct HeaderFooterParagraph {
    pub style: ParagraphStyle,
    pub elements: Vec<HFInline>,
    pub border: Option<CellBorder>,
    /// `w:pBdr` per-side `w:space` offsets in points, which set the gap Word
    /// leaves between the paragraph text and each rule.
    pub border_space: Option<Insets>,
    pub frame: Option<HeaderFooterFrame>,
}

/// Page- or margin-relative positioning for a header/footer paragraph frame.
#[derive(Debug, Clone, PartialEq)]
pub struct HeaderFooterFrame {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub horizontal_anchor: FrameAnchor,
    pub vertical_anchor: FrameAnchor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrameAnchor {
    Page,
    Margin,
    #[default]
    Text,
}

/// A position-relative tab (`w:ptab`) inside header/footer content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionedTab {
    pub alignment: PositionedTabAlignment,
    pub relative_to: PositionedTabRelativeTo,
    pub leader: TabLeader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PositionedTabAlignment {
    Center,
    #[default]
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PositionedTabRelativeTo {
    Indent,
    #[default]
    Margin,
}

/// An inline element within a header or footer paragraph.
#[derive(Debug, Clone)]
pub enum HFInline {
    /// A text run with styling.
    Run(Run),
    /// An inline image embedded in the header or footer part.
    Image(ImageData),
    /// Current page number field, carrying the run properties of the `w:r`
    /// that holds it so the number matches the surrounding literals.
    PageNumber(TextStyle),
    /// Total page count field, styled like [`HFInline::PageNumber(TextStyle::default())`].
    TotalPages(TextStyle),
    /// Alignment tab positioned relative to the paragraph indent or page margin.
    PositionedTab(PositionedTab),
}

/// Block-level content elements.
#[derive(Debug, Clone)]
pub enum Block {
    Paragraph(Paragraph),
    Table(Table),
    Image(ImageData),
    /// Consecutive inline images from one flow paragraph.
    InlineImages(Vec<ImageData>),
    FloatingImage(FloatingImage),
    FloatingTextBox(FloatingTextBox),
    FloatingShape(FloatingShape),
    List(List),
    MathEquation(MathEquation),
    Chart(Chart),
    /// A `TOC` field's result, computed at render time from the document's own
    /// headings or captions.
    TableOfContents(TableOfContents),
    /// A paragraph numbered by a `SEQ` field — a figure or table caption.
    ///
    /// It renders exactly like the paragraph it wraps; the wrapper exists so a
    /// `TOC \a` list can collect it (issue #576).
    Caption(Caption),
    PageBreak,
    ColumnBreak,
}

/// What a `TOC` field collects.
///
/// Word stores the entries it last computed inside the field. A generated
/// document leaves the field dirty and empty for Word to fill on open, so the
/// entries have to be computed rather than read (issue #576).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableOfContents {
    /// `TOC \o "1-3"`: paragraphs whose style carries `w:outlineLvl`, to this
    /// depth.
    Headings { depth: u8 },
    /// `TOC \a "Figure"`: the captions counted by that `SEQ` identifier.
    Captions { identifier: String },
}

/// A caption paragraph and the `SEQ` identifier numbering it.
#[derive(Debug, Clone)]
pub struct Caption {
    /// The `SEQ` identifier — `Figure`, `Table` — whose list collects this.
    pub identifier: String,
    /// The text a `TOC \a` list shows: the caption without the label and the
    /// field's number, which Word leaves out of the list.
    pub entry_text: String,
    pub paragraph: Paragraph,
}

/// What `c:chartSpace/c:spPr/a:ln` asks for around the whole chart area.
///
/// The three cases are visually opposite and the corpus holds all of them, so
/// one unconditional default would put a border on charts that ask for none and
/// the wrong border on charts that ask for their own (#637):
///
/// - `xlsx/poi/WithChart.xlsx` declares no `a:ln` at all — [`Self::Default`].
/// - `xlsx/poi/123233_charts.xlsx` and `pptx/oxp_CU018-Chart-Cached-Data-41.pptx`
///   declare `<a:ln><a:noFill/></a:ln>` — [`Self::Suppressed`].
/// - `xlsx/office2pdf_repository_workbook.xlsx` declares a 9360 EMU `#d9d9d9`
///   line and `pptx/chart-picture-bg.pptx` a 28575 EMU accent one —
///   [`Self::Explicit`].
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ChartAreaOutline {
    /// No `a:ln` at all — Office draws its default thin outline.
    #[default]
    Default,
    /// `<a:ln><a:noFill/></a:ln>` — the file asks for no outline.
    Suppressed,
    /// An explicit line. Either component falls back to the default when the
    /// file leaves it out, or names a colour this parser cannot resolve.
    Explicit {
        /// `a:ln/@w` converted from EMU.
        width_pt: Option<f64>,
        /// The line's `a:solidFill/a:srgbClr`.
        color: Option<Color>,
    },
}

/// A chart extracted from an embedded chart object.
#[derive(Debug, Clone)]
pub struct Chart {
    /// The type of chart (bar, line, pie, etc.).
    pub chart_type: ChartType,
    /// `<c:holeSize val>` for a doughnut, as a percentage of the outer radius.
    /// `None` for every other type (issue #679).
    pub hole_size_percent: Option<u32>,
    /// Optional chart title.
    pub title: Option<String>,
    /// Category labels (x-axis or pie slice names).
    pub categories: Vec<String>,
    /// Data series.
    pub series: Vec<ChartSeries>,
    /// How a category's series share one bar.
    pub grouping: ChartGrouping,
    /// Where the legend sits, from `<c:legendPos>`.
    pub legend_position: LegendPosition,
    /// Whether the chart declares a `<c:legend>` at all, and did not switch it
    /// off with `<c:delete val="1"/>`.
    ///
    /// Separate from `legend_position`, which falls back to a default for
    /// every chart and so cannot distinguish "no legend" from "legend on the
    /// right" (issue #762).
    pub has_legend: bool,
    /// Title of the category axis, from `<c:catAx><c:title>`.
    pub category_axis_title: Option<String>,
    /// Title of the value axis, from `<c:valAx><c:title>`. Office writes it
    /// rotated a quarter turn anticlockwise along the axis.
    pub value_axis_title: Option<String>,
    /// Where the category axis puts its major tick marks, from
    /// `<c:catAx><c:majorTickMark>`.
    pub category_axis_major_tick_mark: AxisTickMark,
    /// Where the value axis puts its major tick marks, from
    /// `<c:valAx><c:majorTickMark>`.
    pub value_axis_major_tick_mark: AxisTickMark,
    /// Whether `<c:catAx><c:delete>` switched the category axis off.
    pub category_axis_deleted: bool,
    /// Whether `<c:valAx><c:delete>` switched the value axis off.
    ///
    /// Office keeps the rest of a switched-off axis' settings — a hidden axis
    /// usually still carries `<c:majorTickMark val="out"/>` — so the flag is
    /// what decides whether the axis is drawn, not the settings beside it.
    pub value_axis_deleted: bool,
    /// How the bars of one category share the band it gets, from
    /// `<c:barChart>`. Charts outside the bar family carry the defaults.
    pub bar_band_layout: BarBandLayout,
    /// `accent1`..`accent6` of the theme the chart's package declares, in that
    /// order, for series that state no fill of their own.
    ///
    /// Empty when the package has no theme, or names fewer than six accents,
    /// in which case the renderer keeps its built-in palette. That palette is
    /// the Office 2013+ one, so a file built on any other theme was recoloured
    /// by it (issue #670).
    pub theme_accent_colors: Vec<Color>,
    /// What the chart area's own outline should be, from
    /// `c:chartSpace/c:spPr/a:ln` (#637).
    pub chart_area_outline: ChartAreaOutline,
}

/// How a bar chart's bars divide the band one category gets, from
/// `<c:barChart><c:gapWidth>` and `<c:barChart><c:overlap>`.
///
/// Both are measured in units of ONE bar's thickness rather than of the band,
/// so the two together decide how thick a bar is: a band holds the cluster its
/// series form plus a `gap_width_percent` gutter beside it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarBandLayout {
    /// `<c:gapWidth>` (`ST_GapAmount`, 0..=500) — the gutter between
    /// neighbouring category bands, as a percentage of one bar's thickness.
    /// 100 makes the gutter exactly as wide as a bar.
    pub gap_width_percent: f64,
    /// `<c:overlap>` (`ST_Overlap`, -100..=100) — how far each clustered
    /// series' bar slides over its predecessor, as a percentage of one bar's
    /// thickness. Negative values push them apart instead.
    pub overlap_percent: f64,
}

impl Default for BarBandLayout {
    /// The values Office draws when a chart declares neither element, which are
    /// also ECMA-376's attribute defaults.
    ///
    /// Measured, not recalled: `tests/fixtures/xlsx/chart_sheet.xlsx` omits both
    /// elements, and Excel 16.0 exports its two clustered series as touching
    /// 42.3pt bars on a 148.1pt band — 148.1/42.3 is 2 series + 150%, and
    /// touching bars are an overlap of 0.
    fn default() -> Self {
        Self {
            gap_width_percent: 150.0,
            overlap_percent: 0.0,
        }
    }
}

/// Which side of an axis line its major tick marks project from, from
/// `<c:majorTickMark>` (`ST_TickMark`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AxisTickMark {
    /// `none` — the axis carries no major tick marks.
    None,
    /// `in` — the ticks reach into the plot area.
    Inside,
    /// `out` — the ticks reach away from the plot area.
    ///
    /// The default is what Office renders for an axis that never mentions tick
    /// marks, not the `cross` ECMA-376 gives the attribute: Excel 16.0 exports
    /// `tests/fixtures/xlsx/WithChart.xlsx` — written by Apache POI without a
    /// single `<c:majorTickMark>` — with outward ticks on both axes.
    #[default]
    Outside,
    /// `cross` — the ticks straddle the axis line, reaching both ways.
    Cross,
}

/// What a chart's data labels print, from `<c:dLbls>`.
///
/// Office joins the enabled parts with `<c:separator>`, defaulting to `"; "`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataLabels {
    /// `<c:showVal>` — the point's own value.
    pub show_value: bool,
    /// `<c:showCatName>` — the category it sits over.
    pub show_category: bool,
    /// `<c:showSerName>` — the series it belongs to.
    pub show_series: bool,
    /// `<c:showPercent>` — its share of the category total.
    pub show_percent: bool,
    /// `<c:separator>` between the enabled parts.
    pub separator: String,
}

impl Default for DataLabels {
    fn default() -> Self {
        Self {
            show_value: false,
            show_category: false,
            show_series: false,
            show_percent: false,
            separator: "; ".to_string(),
        }
    }
}

impl DataLabels {
    /// Whether anything at all is printed.
    pub fn is_empty(&self) -> bool {
        !(self.show_value || self.show_category || self.show_series || self.show_percent)
    }
}

/// Where a chart's legend sits relative to its plot, from `<c:legendPos>`.
///
/// ECMA-376 gives `ST_LegendPos` a default of `r`, which is also where every
/// legend used to be drawn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LegendPosition {
    Bottom,
    Left,
    #[default]
    Right,
    Top,
    TopRight,
}

impl LegendPosition {
    /// Whether entries flow left to right rather than stacking downward.
    /// PowerPoint lays a legend out along the edge it sits on.
    pub fn is_horizontal(self) -> bool {
        matches!(self, LegendPosition::Bottom | LegendPosition::Top)
    }
}

/// How the series of one category are combined, from `<c:grouping>`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ChartGrouping {
    /// Each series gets its own mark side by side, and so the shape a chart
    /// without `<c:grouping>` takes: ECMA-376 defaults `CT_BarGrouping` to
    /// `clustered` and `CT_Grouping` to `standard`, which both mean unstacked.
    #[default]
    Clustered,
    /// A category's series stack into one bar whose length is their total.
    Stacked,
    /// As `Stacked`, with every stack normalised to 100%.
    PercentStacked,
}

/// The type of chart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChartType {
    Bar,
    Column,
    Line,
    Pie,
    /// A pie with a concentric hole; `Chart::hole_size_percent` carries the
    /// inner radius as a percentage of the outer (issue #679).
    Doughnut,
    Area,
    Scatter,
    Other(String),
}

/// A data series within a chart.
#[derive(Debug, Clone)]
pub struct ChartSeries {
    /// Optional series name.
    pub name: Option<String>,
    /// Data values for this series.
    pub values: Vec<f64>,
    /// Fill declared by the series' own `<c:spPr>`. `None` falls back to the
    /// built-in palette.
    pub fill: Option<Color>,
    /// Per-point fills from `<c:dPt>`, indexed by data point. A point's own
    /// fill outranks the series'; entries are `None` where the point declares
    /// none, and the vector may be shorter than `values`.
    pub point_fills: Vec<Option<Color>>,
    /// What this series' `<c:dLbls>` prints beside each point.
    pub data_labels: DataLabels,
}

impl ChartSeries {
    /// The fill for one data point: its own, else the series', else `None` for
    /// the caller to take from the palette.
    pub fn fill_for_point(&self, point_index: usize) -> Option<Color> {
        self.point_fills
            .get(point_index)
            .copied()
            .flatten()
            .or(self.fill)
    }
}

/// A math equation (from OMML or similar).
#[derive(Debug, Clone)]
pub struct MathEquation {
    /// Typst math notation content (without surrounding `$` delimiters).
    pub content: String,
    /// Whether this is a display equation (centered, on its own line) vs inline.
    pub display: bool,
}

/// How text wraps around a floating image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapMode {
    /// Text wraps around the image on both sides (square bounding box).
    Square,
    /// Text wraps tightly around the image contour.
    Tight,
    /// Text appears above and below the image only (no side wrapping).
    TopAndBottom,
    /// Image is behind the text (no wrapping, text flows over).
    Behind,
    /// Image is in front of the text (no wrapping, image covers text).
    InFront,
    /// No text wrapping.
    None,
}

/// A floating image with positioning and text wrap mode.
#[derive(Debug, Clone)]
pub struct FloatingImage {
    pub image: ImageData,
    pub wrap_mode: WrapMode,
    /// Horizontal offset in points from the anchor reference.
    pub offset_x: f64,
    /// Vertical offset in points from the anchor reference.
    pub offset_y: f64,
}

/// A floating text box with positioning, size, and text wrap mode.
#[derive(Debug, Clone)]
pub struct FloatingTextBox {
    pub content: Vec<Block>,
    pub wrap_mode: WrapMode,
    pub width: f64,
    pub height: f64,
    pub padding: Insets,
    pub vertical_align: TextBoxVerticalAlign,
    /// Horizontal offset in points from the anchor reference.
    pub offset_x: f64,
    /// Vertical offset in points from the anchor reference.
    pub offset_y: f64,
}

/// A floating geometric shape (rectangle, line/arrow, ellipse, …) positioned
/// with an anchor offset. Used for DrawingML word-processing shapes (`wps:wsp`)
/// that carry geometry but no text box — these have no docx-rs representation
/// and would otherwise be dropped (issue #176).
#[derive(Debug, Clone)]
pub struct FloatingShape {
    pub shape: Shape,
    /// On-page bounding-box width in points (from `wp:extent`).
    pub width: f64,
    /// On-page bounding-box height in points (from `wp:extent`).
    pub height: f64,
    /// Horizontal offset in points from the anchor reference.
    pub offset_x: f64,
    /// Vertical offset in points from the anchor reference.
    pub offset_y: f64,
    pub wrap_mode: WrapMode,
}

/// Vertical alignment for fixed text box content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextBoxVerticalAlign {
    #[default]
    Top,
    Center,
    Bottom,
}

/// A fixed-position text box with content padding and vertical alignment.
#[derive(Debug, Clone)]
pub struct TextBoxData {
    pub content: Vec<Block>,
    pub padding: Insets,
    pub vertical_align: TextBoxVerticalAlign,
    /// Background fill color for the text box.
    pub fill: Option<Color>,
    /// Opacity from 0.0 (fully transparent) to 1.0 (fully opaque).
    pub opacity: Option<f64>,
    /// Border stroke for the text box.
    pub stroke: Option<BorderSide>,
    /// Shape geometry when the text box originates from a non-rectangular shape
    /// (e.g., `roundRect`, `homePlate`). `None` means default rectangle.
    pub shape_kind: Option<ShapeKind>,
    /// When true, text should not wrap — the content width is unconstrained.
    /// Corresponds to `<a:bodyPr wrap="none"/>` in OOXML.
    pub no_wrap: bool,
    /// Whether the source requested PowerPoint autofit behavior for this box.
    pub auto_fit: bool,
    /// Clockwise text rotation from `<a:bodyPr vert>` ("vert" = 90°,
    /// "vert270" = 270°); the box geometry itself stays unrotated.
    pub text_rotation_deg: Option<f64>,
}

/// The kind of list: ordered (numbered) or unordered (bulleted).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    Ordered,
    Unordered,
}

/// Numbering configuration for a specific list level.
#[derive(Debug, Clone, PartialEq)]
pub struct ListLevelStyle {
    pub kind: ListKind,
    /// Optional Typst numbering pattern derived from Word's lvlText/numFmt.
    pub numbering_pattern: Option<String>,
    /// Whether parent numbers should be shown for nested ordered lists.
    pub full_numbering: bool,
    /// Optional concrete marker text for unordered PPTX bullet lists.
    pub marker_text: Option<String>,
    /// Optional concrete marker presentation resolved from the source format.
    pub marker_style: Option<TextStyle>,
}

/// A list block containing items at various indent levels.
#[derive(Debug, Clone)]
pub struct List {
    pub kind: ListKind,
    pub items: Vec<ListItem>,
    /// Per-level list style overrides. Levels not present fall back to `kind`.
    pub level_styles: BTreeMap<u32, ListLevelStyle>,
}

/// A single list item with content and indent level.
#[derive(Debug, Clone)]
pub struct ListItem {
    pub content: Vec<Paragraph>,
    pub level: u32,
    /// Ordered list item number when this item begins a new numbering run.
    pub start_at: Option<u32>,
}

/// A paragraph consisting of styled text runs.
#[derive(Debug, Clone)]
pub struct Paragraph {
    pub style: ParagraphStyle,
    pub runs: Vec<Run>,
}

/// A run of text with uniform formatting.
#[derive(Debug, Clone)]
pub struct Run {
    pub text: String,
    pub style: TextStyle,
    /// Optional hyperlink URL. When present, the run is rendered as a clickable link.
    pub href: Option<String>,
    /// Optional footnote/endnote content. When present, a footnote marker is emitted and
    /// the content is rendered at the bottom of the page.
    pub footnote: Option<Vec<Run>>,
}

/// A table.
#[derive(Debug, Clone, Default)]
pub struct Table {
    pub rows: Vec<TableRow>,
    pub column_widths: Vec<f64>,
    /// Number of leading rows that should repeat as the table header.
    pub header_row_count: usize,
    /// Number of rows above the repeating header that belong to the header
    /// block but must not repeat. Excel's `_xlnm.Print_Titles` can name a row
    /// below the sheet top; the rows above it print once, on the first page.
    pub non_repeating_header_row_count: usize,
    /// Optional block alignment for the table within the flow.
    pub alignment: Option<Alignment>,
    /// Default cell padding applied by the table when cells don't override it.
    pub default_cell_padding: Option<Insets>,
    /// When true, row heights should be derived from content instead of forced to
    /// the exact source row sizes. PowerPoint often renders slide tables this way.
    pub use_content_driven_row_heights: bool,
    /// Default vertical alignment for cells that don't override it.
    /// Excel prints cells bottom-aligned by default; Word/PowerPoint keep
    /// the renderer default (top).
    pub default_vertical_align: Option<CellVerticalAlign>,
    /// When true, a bottom-aligned cell rests its last line's descender on the
    /// row's bottom inset edge, as Excel prints. Only spreadsheet tables set
    /// this: Word's and PowerPoint's bottom-cell seating is unverified against
    /// native GT, so their emission must not change (issue #618).
    pub seats_bottom_aligned_text_on_descender: bool,
    /// When true, each border paints as a filled band anchored to the nominal
    /// grid boundary (Excel's printed convention, measured on a native Excel
    /// 16.111 probe: `thin` fills `[B, B+1]`, `medium` `[B-1, B+1]`, `thick`
    /// `[B-1, B+2]`) instead of a Typst stroke centred on the boundary
    /// (issue #619). Only spreadsheet tables set this; Word's and
    /// PowerPoint's border-painting conventions are unmeasured against
    /// their native GT, so they keep the centred-stroke path.
    pub paints_borders_inside_boundary: bool,
    /// When true, `<printOptions gridLines="1"/>` asks Excel to print its
    /// gridline hairline on every cell boundary of the printed range, under
    /// any explicit border styling (issue #622). Only spreadsheet tables set
    /// this, and it is honoured only together with
    /// `paints_borders_inside_boundary`, whose boundary-band machinery the
    /// gridlines reuse; Word/PowerPoint tables never print gridlines.
    pub prints_gridlines: bool,
    /// When true, `<printOptions headings="1"/>` prints Excel's row-number
    /// gutter and column-letter strip on every page (issue #623). The XLSX
    /// parser materializes both in the IR — the gutter as a prepended first
    /// column so the numbers flow with row pagination, and the letter strip
    /// as `rows[0]` — and codegen re-emits that first row as a
    /// `table.header(repeat: true)` above any print-title headers and paints
    /// GT's 1pt black print frame on the table's exterior boundaries.
    /// `header_row_count` and `non_repeating_header_row_count` keep counting
    /// from the first row AFTER the strip. Word/PowerPoint tables never set
    /// this.
    pub prints_headings: bool,
}

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub height: Option<f64>,
}

/// Glyphs the parser records in [`TableCell::icon_text`] for Excel's arrow
/// icon sets. The renderer recognizes them to draw Excel's filled arrow shapes
/// instead of a character.
pub const ICON_ARROW_UP: &str = "\u{25B2}"; // ▲ black up-pointing triangle
pub const ICON_ARROW_DOWN: &str = "\u{25BC}"; // ▼ black down-pointing triangle
pub const ICON_ARROW_RIGHT: &str = "\u{25B6}"; // ▶ black right-pointing triangle
pub const ICON_ARROW_UP_RIGHT: &str = "\u{25E5}"; // ◥ black upper-right triangle
pub const ICON_ARROW_DOWN_RIGHT: &str = "\u{25E2}"; // ◢ black lower-right triangle

/// Glyph the parser records for the circular icon sets — traffic lights and
/// signs. Excel draws these as filled discs rather than characters, so the
/// renderer recognizes this marker the same way it does the arrows (#536).
pub const ICON_CIRCLE: &str = "\u{25CF}"; // ● black circle

/// A data bar rendering within a cell (conditional formatting).
#[derive(Debug, Clone)]
pub struct DataBarInfo {
    /// Bar color.
    pub color: Color,
    /// Fill percentage from 0.0 to 1.0.
    pub fill_pct: f64,
}

/// Vertical alignment within a table cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellVerticalAlign {
    Top,
    Center,
    Bottom,
}

/// Insets/padding in points.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Insets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

/// A table cell.
#[derive(Debug, Clone)]
pub struct TableCell {
    pub content: Vec<Block>,
    pub col_span: u32,
    pub row_span: u32,
    pub border: Option<CellBorder>,
    pub background: Option<Color>,
    /// DataBar conditional formatting render info.
    pub data_bar: Option<DataBarInfo>,
    /// IconSet text symbol prepended to cell content.
    pub icon_text: Option<String>,
    /// Fill color of the IconSet symbol (Excel draws icons in band colors).
    pub icon_color: Option<Color>,
    /// Width in points that an unwrapped cell's single line paints across
    /// before it is clipped. `None` when the text fits its column and needs no
    /// clip box.
    ///
    /// Excel never moves a `wrapText="false"` cell's text to a second line, so
    /// this is what varies instead of the line count. A general/left cell paints
    /// on across consecutive empty columns to its right, giving its own column
    /// plus those; a centred or right-aligned cell, and any cell whose neighbour
    /// is occupied, gets its own column width alone and is clipped at its edge.
    pub spill_width: Option<f64>,
    /// Vertical alignment of cell content.
    pub vertical_align: Option<CellVerticalAlign>,
    /// Optional cell padding override in points.
    pub padding: Option<Insets>,
}

impl Default for TableCell {
    fn default() -> Self {
        Self {
            content: Vec::new(),
            col_span: 1,
            row_span: 1,
            border: None,
            background: None,
            data_bar: None,
            icon_text: None,
            icon_color: None,
            spill_width: None,
            vertical_align: None,
            padding: None,
        }
    }
}

/// Cell border specification.
#[derive(Debug, Clone, Default)]
pub struct CellBorder {
    pub top: Option<BorderSide>,
    pub bottom: Option<BorderSide>,
    pub left: Option<BorderSide>,
    pub right: Option<BorderSide>,
}

/// Border line style (dash pattern).
///
/// The first block is the cross-format set: Word `w:val` and Excel border
/// styles map onto it, and so do the three DrawingML presets that share its
/// names. The second block exists because DrawingML has more distinct dash
/// rhythms than that set can name — `lgDash` (8w on) and `sysDash` (3w on)
/// are not the same line as `dash` (4w on), and folding them together renders
/// one preset as another (issue #758). Word and Excel never produce them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderLineStyle {
    #[default]
    Solid,
    /// DrawingML `dash`.
    Dashed,
    /// DrawingML `dot`.
    Dotted,
    /// DrawingML `dashDot`.
    DashDot,
    /// No DrawingML preset maps here; Word `dotDotDash` and its Excel kin do.
    DashDotDot,
    Double,
    None,
    /// DrawingML `sysDot`.
    SystemDot,
    /// DrawingML `sysDash`.
    SystemDash,
    /// DrawingML `lgDash`.
    LargeDash,
    /// DrawingML `sysDashDot`.
    SystemDashDot,
    /// DrawingML `lgDashDot`.
    LargeDashDot,
    /// DrawingML `sysDashDotDot`.
    SystemDashDotDot,
    /// DrawingML `lgDashDotDot`.
    LargeDashDotDot,
}

/// A single border side.
#[derive(Debug, Clone)]
pub struct BorderSide {
    pub width: f64,
    pub color: Color,
    pub style: BorderLineStyle,
}

/// Fractions of the source image cropped away from each edge.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ImageCrop {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl ImageCrop {
    pub fn is_empty(&self) -> bool {
        self.left == 0.0 && self.top == 0.0 && self.right == 0.0 && self.bottom == 0.0
    }
}

/// Image data.
#[derive(Debug, Clone)]
pub struct ImageData {
    pub data: Vec<u8>,
    /// Clockwise rotation in degrees from `a:xfrm/@rot`, about the image's
    /// centre. `None` means upright (issue #682).
    pub rotation_deg: Option<f64>,
    pub format: ImageFormat,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub crop: Option<ImageCrop>,
    /// Optional border stroke around the image.
    pub stroke: Option<BorderSide>,
    /// Horizontal placement inherited from the containing paragraph
    /// (flow documents); None renders at the flow default (left).
    pub alignment: Option<Alignment>,
    /// Clip geometry from the picture's `<a:prstGeom>` (crop to shape).
    pub clip_shape: Option<ImageClipShape>,
    /// Outer shadow effect (`a:effectLst/a:outerShdw` on `p:pic`).
    pub shadow: Option<Shadow>,
    /// Vertical gaps declared by the containing paragraph's `w:spacing`
    /// (flow documents). Word advances a picture paragraph by the picture
    /// plus these, so they have to survive the paragraph being dropped.
    pub paragraph_spacing: Option<ImageParagraphSpacing>,
}

/// The `w:spacing` of the paragraph that held an inline picture, in points.
///
/// Kept apart from [`ImageData::alignment`] because a group of pictures in one
/// paragraph shares a single gap above and below rather than one per picture.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ImageParagraphSpacing {
    pub before: Option<f64>,
    pub after: Option<f64>,
}

/// Supported picture clip geometries (PowerPoint "crop to shape").
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageClipShape {
    /// Rounded rectangle with the corner radius as a fraction of the
    /// shorter side (PowerPoint's roundRect `adj`, default 1/6 ≈ 0.1667).
    RoundedRect(f64),
    Ellipse,
}

/// Supported image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    Bmp,
    Tiff,
    Svg,
}

impl ImageFormat {
    /// Return the file extension for this image format.
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
            Self::Gif => "gif",
            Self::Bmp => "bmp",
            Self::Tiff => "tiff",
            Self::Svg => "svg",
        }
    }
}

/// A node in a SmartArt diagram with hierarchy depth.
#[derive(Debug, Clone, PartialEq)]
pub struct SmartArtNode {
    /// The text content of this node.
    pub text: String,
    /// Depth in the hierarchy (0 = top-level node).
    pub depth: usize,
}

/// SmartArt diagram content extracted from a presentation.
///
/// Contains nodes extracted from the SmartArt data model with hierarchy
/// information derived from the connection list.
/// Rendered as an indented tree or numbered steps since full SmartArt
/// layout engines are not feasible in a pure-Rust converter.
#[derive(Debug, Clone)]
pub struct SmartArt {
    /// Nodes extracted from SmartArt data points with hierarchy depth.
    pub items: Vec<SmartArtNode>,
}

/// A single stop in a gradient fill.
#[derive(Debug, Clone)]
pub struct GradientStop {
    /// Position along the gradient axis, from 0.0 (start) to 1.0 (end).
    pub position: f64,
    /// Color at this stop.
    pub color: Color,
}

/// A linear gradient fill.
#[derive(Debug, Clone)]
pub struct GradientFill {
    /// Gradient color stops, ordered by position.
    pub stops: Vec<GradientStop>,
    /// Angle of the linear gradient in degrees (0 = left-to-right, 90 = top-to-bottom).
    pub angle: f64,
}

/// One of the preset DrawingML patterns from `ST_PresetPatternVal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternPreset {
    Percent5,
    Percent10,
    Percent20,
    Percent25,
    Percent30,
    Percent40,
    Percent50,
    Percent60,
    Percent70,
    Percent75,
    Percent80,
    Percent90,
    Horizontal,
    Vertical,
    LightHorizontal,
    LightVertical,
    DarkHorizontal,
    DarkVertical,
    NarrowHorizontal,
    NarrowVertical,
    DashedHorizontal,
    DashedVertical,
    Cross,
    DownwardDiagonal,
    UpwardDiagonal,
    LightDownwardDiagonal,
    LightUpwardDiagonal,
    DarkDownwardDiagonal,
    DarkUpwardDiagonal,
    WideDownwardDiagonal,
    WideUpwardDiagonal,
    DashedDownwardDiagonal,
    DashedUpwardDiagonal,
    DiagonalCross,
    SmallCheck,
    LargeCheck,
    SmallGrid,
    LargeGrid,
    DotGrid,
    SmallConfetti,
    LargeConfetti,
    HorizontalBrick,
    DiagonalBrick,
    SolidDiamond,
    OpenDiamond,
    DottedDiamond,
    Plaid,
    Sphere,
    Weave,
    Divot,
    Shingle,
    Wave,
    Trellis,
    ZigZag,
}

impl PatternPreset {
    /// Every preset defined by DrawingML's `ST_PresetPatternVal`.
    pub const ALL: [Self; 54] = [
        Self::Percent5,
        Self::Percent10,
        Self::Percent20,
        Self::Percent25,
        Self::Percent30,
        Self::Percent40,
        Self::Percent50,
        Self::Percent60,
        Self::Percent70,
        Self::Percent75,
        Self::Percent80,
        Self::Percent90,
        Self::Horizontal,
        Self::Vertical,
        Self::LightHorizontal,
        Self::LightVertical,
        Self::DarkHorizontal,
        Self::DarkVertical,
        Self::NarrowHorizontal,
        Self::NarrowVertical,
        Self::DashedHorizontal,
        Self::DashedVertical,
        Self::Cross,
        Self::DownwardDiagonal,
        Self::UpwardDiagonal,
        Self::LightDownwardDiagonal,
        Self::LightUpwardDiagonal,
        Self::DarkDownwardDiagonal,
        Self::DarkUpwardDiagonal,
        Self::WideDownwardDiagonal,
        Self::WideUpwardDiagonal,
        Self::DashedDownwardDiagonal,
        Self::DashedUpwardDiagonal,
        Self::DiagonalCross,
        Self::SmallCheck,
        Self::LargeCheck,
        Self::SmallGrid,
        Self::LargeGrid,
        Self::DotGrid,
        Self::SmallConfetti,
        Self::LargeConfetti,
        Self::HorizontalBrick,
        Self::DiagonalBrick,
        Self::SolidDiamond,
        Self::OpenDiamond,
        Self::DottedDiamond,
        Self::Plaid,
        Self::Sphere,
        Self::Weave,
        Self::Divot,
        Self::Shingle,
        Self::Wave,
        Self::Trellis,
        Self::ZigZag,
    ];

    /// Parse the serialized value of DrawingML's `ST_PresetPatternVal`.
    pub(crate) fn from_ooxml(value: &str) -> Option<Self> {
        Some(match value {
            "pct5" => Self::Percent5,
            "pct10" => Self::Percent10,
            "pct20" => Self::Percent20,
            "pct25" => Self::Percent25,
            "pct30" => Self::Percent30,
            "pct40" => Self::Percent40,
            "pct50" => Self::Percent50,
            "pct60" => Self::Percent60,
            "pct70" => Self::Percent70,
            "pct75" => Self::Percent75,
            "pct80" => Self::Percent80,
            "pct90" => Self::Percent90,
            "horz" => Self::Horizontal,
            "vert" => Self::Vertical,
            "ltHorz" => Self::LightHorizontal,
            "ltVert" => Self::LightVertical,
            "dkHorz" => Self::DarkHorizontal,
            "dkVert" => Self::DarkVertical,
            "narHorz" => Self::NarrowHorizontal,
            "narVert" => Self::NarrowVertical,
            "dashHorz" => Self::DashedHorizontal,
            "dashVert" => Self::DashedVertical,
            "cross" => Self::Cross,
            "dnDiag" => Self::DownwardDiagonal,
            "upDiag" => Self::UpwardDiagonal,
            "ltDnDiag" => Self::LightDownwardDiagonal,
            "ltUpDiag" => Self::LightUpwardDiagonal,
            "dkDnDiag" => Self::DarkDownwardDiagonal,
            "dkUpDiag" => Self::DarkUpwardDiagonal,
            "wdDnDiag" => Self::WideDownwardDiagonal,
            "wdUpDiag" => Self::WideUpwardDiagonal,
            "dashDnDiag" => Self::DashedDownwardDiagonal,
            "dashUpDiag" => Self::DashedUpwardDiagonal,
            "diagCross" => Self::DiagonalCross,
            "smCheck" => Self::SmallCheck,
            "lgCheck" => Self::LargeCheck,
            "smGrid" => Self::SmallGrid,
            "lgGrid" => Self::LargeGrid,
            "dotGrid" => Self::DotGrid,
            "smConfetti" => Self::SmallConfetti,
            "lgConfetti" => Self::LargeConfetti,
            "horzBrick" => Self::HorizontalBrick,
            "diagBrick" => Self::DiagonalBrick,
            "solidDmnd" => Self::SolidDiamond,
            "openDmnd" => Self::OpenDiamond,
            "dotDmnd" => Self::DottedDiamond,
            "plaid" => Self::Plaid,
            "sphere" => Self::Sphere,
            "weave" => Self::Weave,
            "divot" => Self::Divot,
            "shingle" => Self::Shingle,
            "wave" => Self::Wave,
            "trellis" => Self::Trellis,
            "zigZag" => Self::ZigZag,
            _ => return None,
        })
    }
}

/// A DrawingML preset pattern with foreground and background colors.
#[derive(Debug, Clone)]
pub struct PatternFill {
    pub preset: PatternPreset,
    pub foreground: Color,
    pub background: Color,
}

/// An outer shadow effect on a shape.
#[derive(Debug, Clone)]
pub struct Shadow {
    /// Blur radius in points.
    pub blur_radius: f64,
    /// Distance from the shape in points.
    pub distance: f64,
    /// Direction angle in degrees (0 = right, 90 = down, 180 = left, 270 = up).
    pub direction: f64,
    /// Shadow color.
    pub color: Color,
    /// Opacity from 0.0 (fully transparent) to 1.0 (fully opaque).
    pub opacity: f64,
}

/// Basic geometric shape.
#[derive(Debug, Clone)]
pub struct Shape {
    pub kind: ShapeKind,
    pub fill: Option<Color>,
    /// Gradient fill for the shape (takes precedence over solid fill when present).
    pub gradient_fill: Option<GradientFill>,
    /// DrawingML preset pattern fill (takes precedence over gradient and solid fills).
    pub pattern_fill: Option<PatternFill>,
    pub stroke: Option<BorderSide>,
    /// Rotation angle in degrees (clockwise).
    pub rotation_deg: Option<f64>,
    /// Opacity from 0.0 (fully transparent) to 1.0 (fully opaque).
    pub opacity: Option<f64>,
    /// Outer shadow effect.
    pub shadow: Option<Shadow>,
}

/// Shape types.
#[derive(Debug, Clone)]
pub enum ShapeKind {
    Rectangle,
    Ellipse,
    /// Straight line from `(x1,y1)` to `(x2,y2)` in points, relative to element's top-left.
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        head_end: ArrowHead,
        tail_end: ArrowHead,
    },
    /// Multi-segment polyline in points, relative to element's top-left.
    Polyline {
        points: Vec<(f64, f64)>,
        head_end: ArrowHead,
        tail_end: ArrowHead,
    },
    /// Rectangle with rounded corners. `radius_fraction` is relative to `min(width, height)`.
    RoundedRectangle {
        radius_fraction: f64,
    },
    /// Arbitrary polygon defined by vertices normalized to 0.0–1.0 relative to the bounding box.
    Polygon {
        vertices: Vec<(f64, f64)>,
    },
}

/// Arrowhead decoration on a line endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrowHead {
    #[default]
    None,
    Triangle,
}

#[cfg(test)]
#[path = "elements_tests.rs"]
mod tests;
