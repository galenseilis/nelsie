use crate::model::{
    Color, Drawing, FontData, Node, NodeChild, NodeContent, NodeId, Span, Step, Stroke, StyledLine,
    StyledText, TextAlign, TextStyle,
};
use crate::render::layout::{ComputedLayout, LayoutContext};
use crate::render::RenderConfig;

use std::collections::BTreeSet;

use std::sync::Arc;

use crate::render::counters::replace_counters;
use crate::render::image::render_image_to_canvas;
use crate::render::paths::{create_arrow, path_from_rect, path_to_svg};

use crate::common::Rectangle;
use crate::parsers::SimpleXmlWriter;
use crate::render::canvas::{Canvas, CanvasItem};
use crate::render::pathbuilder::PathBuilder;
use crate::render::text::{render_text_to_canvas, render_text_to_svg};
use svg2pdf::usvg;

pub(crate) struct RenderContext<'a> {
    config: &'a RenderConfig<'a>,
    z_level: i32,
    layout: &'a ComputedLayout,
    canvas: &'a mut Canvas,
}

impl From<&Color> for usvg::Color {
    fn from(value: &Color) -> Self {
        let c: svgtypes::Color = value.into();
        usvg::Color::new_rgb(c.red, c.green, c.blue)
    }
}

fn draw_debug_frame(
    rect: &Rectangle,
    name: &str,
    font: &Arc<FontData>,
    color: &Color,
    canvas: &mut Canvas,
) {
    let mut xml = SimpleXmlWriter::new();
    let mut path = PathBuilder::new(
        Some(Stroke {
            color: color.clone(),
            width: 1.0,
            dash_array: Some(vec![5.0, 2.5]),
            dash_offset: 0.0,
        }),
        None,
    );
    path.rect(&Rectangle {
        x: rect.x,
        y: rect.y,
        width: rect.width.max(1.0),
        height: rect.height.max(1.0),
    });
    path.write_svg(&mut xml);

    let text = if name.is_empty() {
        format!("[{}x{}]", rect.width, rect.height)
    } else {
        format!("{} [{}x{}]", name, rect.width, rect.height)
    };
    let styled_text = StyledText {
        styled_lines: vec![StyledLine {
            spans: vec![Span {
                length: text.len() as u32,
                style_idx: 0,
            }],
            text,
        }],
        styles: vec![TextStyle {
            font: font.clone(),
            stroke: None,
            color: Some(color.clone()),
            size: 8.0,
            line_spacing: 0.0,
            italic: false,
            stretch: Default::default(),
            weight: 700,
            kerning: true,
        }],
        default_font_size: 8.0,
        default_line_spacing: 0.0,
    };
    render_text_to_svg(
        &mut xml,
        &styled_text,
        rect.x + 2.0,
        rect.y + 3.0,
        TextAlign::Start,
    );
    canvas.add(CanvasItem::SvgChunk(xml.into_string()));
    /*svg_node.children.push(render_text(
        &styled_text,
        rect.x + 2.0,
        rect.y + 3.0,
        TextAlign::Start,
    ));*/
}

impl<'a> RenderContext<'a> {
    pub fn new(
        config: &'a RenderConfig<'a>,
        z_level: i32,
        layout: &'a ComputedLayout,
        canvas: &'a mut Canvas,
    ) -> RenderContext<'a> {
        RenderContext {
            config,
            z_level,
            layout,
            canvas,
        }
    }

    fn render_helper(&mut self, mut step: Step, node: &Node) {
        // active is before step replacement!
        if !node.active.at_step(step) {
            return;
        }
        if let Some(s) = node.replace_steps.get(&step) {
            step = *s;
        }
        if !node.show.at_step(step) {
            return;
        }
        let is_current_z_level = *node.z_level.at_step(step) == self.z_level;
        if is_current_z_level {
            if let Some(color) = &node.bg_color.at_step(step) {
                let rect = &self.layout.node_layout(node.node_id).unwrap().rect;
                let border_radius = *node.border_radius.at_step(step);
                let mut path = PathBuilder::new(None, Some(color.clone()));
                path_from_rect(&mut path, rect, border_radius);
                let mut xml = SimpleXmlWriter::new();
                path.write_svg(&mut xml);
                self.canvas.add(CanvasItem::SvgChunk(xml.into_string()))
            }

            if let Some(content) = &node.content {
                let rect = &self.layout.node_layout(node.node_id).unwrap().rect;
                match content {
                    NodeContent::Text(text) => {
                        let mut t = text.text_style_at_step(step);
                        if text.parse_counters {
                            // Here we do not "step" but "self.config.step" as we want to escape "replace_steps"
                            // for counters
                            replace_counters(
                                self.config.counter_values,
                                &mut t,
                                self.config.slide_id,
                                self.config.step,
                            );
                        }
                        render_text_to_canvas(&t, rect, text.text_align, self.canvas);
                    }
                    NodeContent::Image(image) => {
                        render_image_to_canvas(image, step, rect, self.canvas)
                    }
                }
            }

            if let Some(color) = &node.debug_layout {
                let rect = &self.layout.node_layout(node.node_id).unwrap().rect;
                draw_debug_frame(
                    rect,
                    &node.name,
                    self.config.default_font,
                    color,
                    self.canvas,
                );
            }
        }

        for child in &node.children {
            match child {
                NodeChild::Node(node) => self.render_helper(step, node),
                NodeChild::Draw(draw) => {
                    if is_current_z_level {
                        self.draw(step, node.node_id, draw)
                    }
                }
            }
        }
    }

    fn draw(&mut self, step: Step, parent_id: NodeId, drawing: &Drawing) {
        let paths = drawing.paths.at_step(step);
        if paths.is_empty() {
            return;
        }
        let mut xml = SimpleXmlWriter::new();
        for path in drawing.paths.at_step(step) {
            path_to_svg(&mut xml, self.layout, parent_id, path);
            create_arrow(&mut xml, self.layout, parent_id, path, true);
            create_arrow(&mut xml, self.layout, parent_id, path, false);
        }
        self.canvas.add(CanvasItem::SvgChunk(xml.into_string()))
    }

    pub(crate) fn render_to_svg(mut self, step: Step, node: &Node) {
        self.render_helper(step, node);
    }
}

pub(crate) fn render_to_canvas(render_cfg: &RenderConfig) -> Canvas {
    log::debug!("Creating layout");
    let layout_builder = LayoutContext::new(render_cfg);
    let layout = layout_builder.compute_layout(render_cfg.slide, render_cfg.step);

    log::debug!("Layout {:?}", layout);

    let mut z_levels = BTreeSet::new();
    render_cfg.slide.node.collect_z_levels(&mut z_levels);

    log::debug!("Rendering to canvas");
    let slide = &render_cfg.slide;
    let mut canvas = Canvas::new(slide.width, slide.height, slide.bg_color.clone());

    for z_level in z_levels {
        let render_ctx = RenderContext::new(render_cfg, z_level, &layout, &mut canvas);
        render_ctx.render_to_svg(render_cfg.step, &render_cfg.slide.node);
    }
    canvas
}
