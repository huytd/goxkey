use crate::input::TypingMethod;
use druid::{
    kurbo::{BezPath, Circle, RoundedRect},
    piet::{FontFamily, Text, TextLayout, TextLayoutBuilder},
    BoxConstraints, Color, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx,
    Point, Rect, RenderContext, Size, UpdateCtx, Widget, WidgetPod,
};

use super::{
    colors::{
        BADGE_BG, BADGE_BORDER, BADGE_EN_BG, BADGE_EN_BORDER, BADGE_VI_BG, BADGE_VI_BORDER,
        DIVIDER, GREEN, GREEN_BG, TEXT_PRIMARY, TEXT_SECONDARY,
    },
    data::UIDataAdapter,
    selectors::TOGGLE_APP_MODE,
};

// ══════════════════════════════════════════════════════════════════════════════
// ToggleSwitch
// ══════════════════════════════════════════════════════════════════════════════

pub(super) struct ToggleSwitch;

impl Widget<bool> for ToggleSwitch {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut bool, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.request_paint();
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    *data = !*data;
                    ctx.request_paint();
                }
            }
            _ => {}
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &bool, _env: &Env) {
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &bool, data: &bool, _env: &Env) {
        if old_data != data {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        _bc: &BoxConstraints,
        _data: &bool,
        _env: &Env,
    ) -> Size {
        Size::new(36.0, 20.0)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &bool, _env: &Env) {
        let size = ctx.size();
        let radius = size.height / 2.0;
        let track_rect = RoundedRect::new(0.0, 0.0, size.width, size.height, radius);
        let track_color = if *data {
            GREEN
        } else {
            Color::rgb8(187, 187, 187)
        };
        ctx.fill(track_rect, &track_color);

        let knob_r = radius - 2.0;
        let knob_x = if *data { size.width - radius } else { radius };
        ctx.fill(
            Circle::new(Point::new(knob_x, size.height / 2.0), knob_r),
            &Color::WHITE,
        );
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// StyledCheckbox
// ══════════════════════════════════════════════════════════════════════════════

pub(super) struct StyledCheckbox;

impl Widget<bool> for StyledCheckbox {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut bool, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.request_paint();
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    *data = !*data;
                    ctx.request_paint();
                }
            }
            _ => {}
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &bool, _env: &Env) {
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &bool, data: &bool, _env: &Env) {
        if old_data != data {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        _bc: &BoxConstraints,
        _data: &bool,
        _env: &Env,
    ) -> Size {
        Size::new(18.0, 18.0)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &bool, _env: &Env) {
        let box_rect = RoundedRect::new(1.0, 1.0, 17.0, 17.0, 4.0);
        if *data {
            ctx.fill(box_rect, &GREEN);
            let mut path = BezPath::new();
            path.move_to((3.5, 9.0));
            path.line_to((7.0, 12.5));
            path.line_to((14.0, 5.5));
            ctx.stroke(path, &Color::WHITE, 1.8);
        } else {
            ctx.fill(box_rect, &Color::WHITE);
            ctx.stroke(box_rect, &Color::rgb8(204, 204, 204), 1.0);
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// SegmentedControl
// ══════════════════════════════════════════════════════════════════════════════

pub(super) struct SegmentedControl {
    options: Vec<(String, TypingMethod)>,
    rects: Vec<Rect>,
}

impl SegmentedControl {
    pub(super) fn new(options: Vec<(&str, TypingMethod)>) -> Self {
        Self {
            options: options
                .into_iter()
                .map(|(s, m)| (s.to_string(), m))
                .collect(),
            rects: Vec::new(),
        }
    }
}

impl Widget<TypingMethod> for SegmentedControl {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut TypingMethod, _env: &Env) {
        if let Event::MouseDown(mouse) = event {
            for (i, rect) in self.rects.iter().enumerate() {
                if rect.contains(mouse.pos) {
                    *data = self.options[i].1;
                    ctx.request_paint();
                    break;
                }
            }
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &TypingMethod,
        _env: &Env,
    ) {
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &TypingMethod,
        data: &TypingMethod,
        _env: &Env,
    ) {
        if old_data != data {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &TypingMethod,
        _env: &Env,
    ) -> Size {
        let w = bc.max().width;
        let h = 34.0;
        let n = self.options.len() as f64;
        let gap = 8.0;
        let btn_w = (w - gap * (n - 1.0)) / n;
        self.rects = (0..self.options.len())
            .map(|i| {
                let x = i as f64 * (btn_w + gap);
                Rect::new(x, 0.0, x + btn_w, h)
            })
            .collect();
        Size::new(w, h)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &TypingMethod, _env: &Env) {
        for (i, (label, method)) in self.options.iter().enumerate() {
            let rect = self.rects[i];
            let is_active = method == data;
            let rr = RoundedRect::new(rect.x0, rect.y0, rect.x1, rect.y1, 8.0);

            ctx.fill(rr, &Color::WHITE);
            ctx.stroke(rr, &Color::rgb8(221, 221, 221), 1.5);

            if is_active {
                ctx.fill(rr, &GREEN_BG);
                ctx.stroke(rr, &GREEN, 1.5);
            }

            // Label text
            let text_color = if is_active {
                GREEN
            } else {
                Color::rgb8(136, 136, 136)
            };
            let layout = ctx
                .text()
                .new_text_layout(label.clone())
                .font(FontFamily::SYSTEM_UI, 13.0)
                .text_color(text_color)
                .build()
                .unwrap();
            let text_x = rect.x0 + (rect.width() - layout.size().width) / 2.0 + 7.0;
            let text_y = rect.y0 + (rect.height() - layout.size().height) / 2.0 - 1.0;
            ctx.draw_text(&layout, (text_x, text_y));

            // Radio dot
            let dot_cx = text_x - 14.0;
            let dot_cy = rect.y0 + rect.height() / 2.0;
            let ring_color = if is_active {
                GREEN
            } else {
                Color::rgb8(187, 187, 187)
            };
            ctx.stroke(Circle::new((dot_cx, dot_cy), 5.0), &ring_color, 1.5);
            if is_active {
                ctx.fill(Circle::new((dot_cx, dot_cy), 2.5), &GREEN);
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// TabBar
// ══════════════════════════════════════════════════════════════════════════════

pub(super) struct TabBar {
    tab_rects: Vec<Rect>,
}

impl TabBar {
    pub(super) fn new() -> Self {
        Self {
            tab_rects: Vec::new(),
        }
    }

    fn draw_icon_general(ctx: &mut PaintCtx, cx: f64, cy: f64, color: &Color) {
        for (w, y_off) in [(14.0, -4.5), (9.0, 0.0), (11.0, 4.5)] {
            let rr = RoundedRect::new(
                cx - 7.0,
                cy + y_off - 1.5,
                cx - 7.0 + w,
                cy + y_off + 1.5,
                1.5,
            );
            ctx.fill(rr, color);
        }
    }

    fn draw_icon_apps(ctx: &mut PaintCtx, cx: f64, cy: f64, color: &Color) {
        for (dx, dy) in [(-4.5, -4.5), (1.5, -4.5), (-4.5, 1.5), (1.5, 1.5)] {
            let rr = RoundedRect::new(cx + dx, cy + dy, cx + dx + 4.5, cy + dy + 4.5, 1.0);
            ctx.fill(rr, color);
        }
    }

    fn draw_icon_text_expansion(ctx: &mut PaintCtx, cx: f64, cy: f64, color: &Color) {
        // "{" left brace
        let mut brace = BezPath::new();
        brace.move_to((cx - 9.0, cy - 4.5));
        brace.line_to((cx - 11.0, cy - 4.5));
        brace.line_to((cx - 11.0, cy - 1.5));
        brace.line_to((cx - 13.0, cy));
        brace.line_to((cx - 11.0, cy + 1.5));
        brace.line_to((cx - 11.0, cy + 4.5));
        brace.line_to((cx - 9.0, cy + 4.5));
        ctx.stroke(brace, color, 1.3);
        // Arrow →
        let mut arrow = BezPath::new();
        arrow.move_to((cx - 6.0, cy));
        arrow.line_to((cx + 2.0, cy));
        arrow.move_to((cx - 1.0, cy - 2.5));
        arrow.line_to((cx + 2.5, cy));
        arrow.line_to((cx - 1.0, cy + 2.5));
        ctx.stroke(arrow, color, 1.3);
        // "}" right brace
        let mut brace2 = BezPath::new();
        brace2.move_to((cx + 5.0, cy - 4.5));
        brace2.line_to((cx + 7.0, cy - 4.5));
        brace2.line_to((cx + 7.0, cy - 1.5));
        brace2.line_to((cx + 9.0, cy));
        brace2.line_to((cx + 7.0, cy + 1.5));
        brace2.line_to((cx + 7.0, cy + 4.5));
        brace2.line_to((cx + 5.0, cy + 4.5));
        ctx.stroke(brace2, color, 1.3);
    }
}

impl Widget<u32> for TabBar {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut u32, _env: &Env) {
        if let Event::MouseDown(mouse) = event {
            for (i, rect) in self.tab_rects.iter().enumerate() {
                if rect.contains(mouse.pos) {
                    *data = i as u32;
                    ctx.request_paint();
                    break;
                }
            }
        }
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &u32, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &u32, data: &u32, _env: &Env) {
        if old_data != data {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &u32,
        _env: &Env,
    ) -> Size {
        let w = bc.max().width;
        let h = 58.0;
        let tab_w = w / 3.0;
        self.tab_rects = (0..3)
            .map(|i| Rect::new(i as f64 * tab_w, 0.0, (i + 1) as f64 * tab_w, h))
            .collect();
        Size::new(w, h)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &u32, _env: &Env) {
        use druid::kurbo::Line;
        let size = ctx.size();
        ctx.stroke(
            Line::new((0.0, size.height), (size.width, size.height)),
            &Color::rgb8(221, 221, 221),
            0.5,
        );

        let labels = ["General", "Apps", "Text Expansion"];
        let icon_fns: [fn(&mut PaintCtx, f64, f64, &Color); 3] = [
            TabBar::draw_icon_general,
            TabBar::draw_icon_apps,
            TabBar::draw_icon_text_expansion,
        ];

        for (i, rect) in self.tab_rects.iter().enumerate() {
            let is_active = i as u32 == *data;
            let color = if is_active {
                GREEN
            } else {
                Color::rgb8(153, 153, 153)
            };
            let cx = rect.x0 + rect.width() / 2.0;
            let icon_cy = rect.y0 + 18.0;

            icon_fns[i](ctx, cx, icon_cy, &color);

            let layout = ctx
                .text()
                .new_text_layout(labels[i])
                .font(FontFamily::SYSTEM_UI, 10.0)
                .text_color(color.clone())
                .build()
                .unwrap();
            ctx.draw_text(&layout, (cx - layout.size().width / 2.0, icon_cy + 11.0));

            if is_active {
                ctx.fill(
                    Rect::new(rect.x0, size.height - 2.0, rect.x1, size.height),
                    &GREEN,
                );
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// KeyBadge
// ══════════════════════════════════════════════════════════════════════════════

pub(super) struct KeyBadge {
    label: String,
}

impl KeyBadge {
    pub(super) fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

impl Widget<()> for KeyBadge {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut (), _env: &Env) {}
    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &(), _env: &Env) {}
    fn update(&mut self, _ctx: &mut UpdateCtx, _old: &(), _data: &(), _env: &Env) {}

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        _bc: &BoxConstraints,
        _data: &(),
        _env: &Env,
    ) -> Size {
        let char_w = self.label.chars().count() as f64 * 8.0;
        Size::new((char_w + 14.0).max(26.0), 24.0)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &(), _env: &Env) {
        let size = ctx.size();
        let rr = RoundedRect::new(0.0, 0.0, size.width, size.height, 5.0);
        ctx.fill(rr, &BADGE_BG);
        ctx.stroke(rr, &BADGE_BORDER, 0.5);

        let layout = ctx
            .text()
            .new_text_layout(self.label.clone())
            .font(FontFamily::SYSTEM_UI, 12.0)
            .text_color(Color::rgb8(85, 85, 85))
            .build()
            .unwrap();
        let lw = layout.size().width;
        let lh = layout.size().height;
        ctx.draw_text(&layout, ((size.width - lw) / 2.0, (size.height - lh) / 2.0));
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// HotkeyBadgesWidget
// ══════════════════════════════════════════════════════════════════════════════

pub(super) struct HotkeyBadgesWidget {
    badges: Vec<WidgetPod<(), KeyBadge>>,
    last_display: String,
}

impl HotkeyBadgesWidget {
    pub(super) fn new() -> Self {
        Self {
            badges: Vec::new(),
            last_display: String::new(),
        }
    }

    fn rebuild_badges(&mut self, display: &str) {
        self.badges = display
            .split_whitespace()
            .map(|token| WidgetPod::new(KeyBadge::new(token)))
            .collect();
        self.last_display = display.to_string();
    }
}

impl Widget<UIDataAdapter> for HotkeyBadgesWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut UIDataAdapter, env: &Env) {
        for badge in &mut self.badges {
            badge.event(ctx, event, &mut (), env);
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &UIDataAdapter,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            self.rebuild_badges(&data.hotkey_display);
        }
        for badge in &mut self.badges {
            badge.lifecycle(ctx, event, &(), env);
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        _old: &UIDataAdapter,
        data: &UIDataAdapter,
        env: &Env,
    ) {
        if data.hotkey_display != self.last_display {
            self.rebuild_badges(&data.hotkey_display);
            ctx.children_changed();
        }
        for badge in &mut self.badges {
            badge.update(ctx, &(), env);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &UIDataAdapter,
        env: &Env,
    ) -> Size {
        let gap = 4.0;
        let mut x = 0.0;
        let mut max_h = 0.0f64;
        let loose = bc.loosen();
        for badge in &mut self.badges {
            let s = badge.layout(ctx, &loose, &(), env);
            badge.set_origin(ctx, Point::new(x, 0.0));
            x += s.width + gap;
            max_h = max_h.max(s.height);
        }
        Size::new((x - gap).max(0.0), max_h.max(24.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &UIDataAdapter, env: &Env) {
        for badge in &mut self.badges {
            badge.paint(ctx, &(), env);
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// MacroListWidget
// ══════════════════════════════════════════════════════════════════════════════

pub(super) struct MacroListWidget {
    row_rects: Vec<Rect>,
}

const MACRO_ROW_HEIGHT: f64 = 44.0;

impl MacroListWidget {
    pub(super) fn new() -> Self {
        Self { row_rects: Vec::new() }
    }
}

impl Widget<UIDataAdapter> for MacroListWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut UIDataAdapter, _env: &Env) {
        if let Event::MouseDown(mouse) = event {
            for (i, rect) in self.row_rects.iter().enumerate() {
                if rect.contains(mouse.pos) {
                    data.selected_macro_index = i as i32;
                    ctx.request_paint();
                    break;
                }
            }
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &UIDataAdapter,
        _env: &Env,
    ) {
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &UIDataAdapter,
        data: &UIDataAdapter,
        _env: &Env,
    ) {
        if old_data.macro_table != data.macro_table {
            ctx.request_layout();
        } else if old_data.selected_macro_index != data.selected_macro_index {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &UIDataAdapter,
        _env: &Env,
    ) -> Size {
        let n = data.macro_table.len();
        let w = bc.max().width;
        self.row_rects = (0..n)
            .map(|i| {
                Rect::new(0.0, i as f64 * MACRO_ROW_HEIGHT, w, (i + 1) as f64 * MACRO_ROW_HEIGHT)
            })
            .collect();
        Size::new(w, (n as f64 * MACRO_ROW_HEIGHT).max(0.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &UIDataAdapter, _env: &Env) {
        let size = ctx.size();

        for (i, entry) in data.macro_table.iter().enumerate() {
            let rect = self.row_rects[i];
            let is_selected = data.selected_macro_index == i as i32;

            if is_selected {
                ctx.fill(
                    RoundedRect::new(rect.x0, rect.y0, rect.x1, rect.y1, 0.0),
                    &Color::rgba8(0, 0, 0, 8),
                );
            }

            if i > 0 {
                ctx.fill(
                    Rect::new(14.0, rect.y0, size.width - 14.0, rect.y0 + 0.5),
                    &DIVIDER,
                );
            }

            // "From" label (shorthand)
            let from_layout = ctx
                .text()
                .new_text_layout(entry.from.clone())
                .font(FontFamily::SYSTEM_UI, 13.0)
                .text_color(TEXT_PRIMARY)
                .build()
                .unwrap();
            ctx.draw_text(
                &from_layout,
                (14.0, rect.y0 + (MACRO_ROW_HEIGHT - from_layout.size().height) / 2.0),
            );

            // Arrow "→" separator
            let arrow_layout = ctx
                .text()
                .new_text_layout("→")
                .font(FontFamily::SYSTEM_UI, 12.0)
                .text_color(TEXT_SECONDARY)
                .build()
                .unwrap();
            let arrow_x = size.width / 2.0 - arrow_layout.size().width / 2.0;
            ctx.draw_text(
                &arrow_layout,
                (arrow_x, rect.y0 + (MACRO_ROW_HEIGHT - arrow_layout.size().height) / 2.0),
            );

            // "To" label (replacement)
            let to_layout = ctx
                .text()
                .new_text_layout(entry.to.clone())
                .font(FontFamily::SYSTEM_UI, 13.0)
                .text_color(TEXT_PRIMARY)
                .build()
                .unwrap();
            let to_x = size.width / 2.0 + 20.0;
            ctx.draw_text(
                &to_layout,
                (to_x, rect.y0 + (MACRO_ROW_HEIGHT - to_layout.size().height) / 2.0),
            );
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// AppsListWidget
// ══════════════════════════════════════════════════════════════════════════════

pub(super) struct CombinedAppEntry {
    pub(super) display_name: String,
    pub(super) full_name: String,
    pub(super) is_vn: bool,
}

pub(super) struct AppsListWidget {
    row_rects: Vec<Rect>,
    badge_rects: Vec<Rect>,
    avatar_colors: Vec<Color>,
}

const ROW_HEIGHT: f64 = 52.0;

impl AppsListWidget {
    pub(super) fn new() -> Self {
        Self {
            row_rects: Vec::new(),
            badge_rects: Vec::new(),
            avatar_colors: vec![
                Color::rgb8(196, 60, 48),
                Color::rgb8(72, 163, 101),
                Color::rgb8(58, 115, 199),
                Color::rgb8(133, 86, 178),
                Color::rgb8(203, 131, 46),
            ],
        }
    }

    pub(super) fn build_entries(data: &UIDataAdapter) -> Vec<CombinedAppEntry> {
        let to_entry = |e: &crate::ui::data::AppEntry, is_vn: bool| CombinedAppEntry {
            display_name: std::path::Path::new(&e.name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&e.name)
                .to_string(),
            full_name: e.name.clone(),
            is_vn,
        };
        let mut entries: Vec<CombinedAppEntry> = data
            .vn_apps
            .iter()
            .map(|e| to_entry(e, true))
            .chain(data.en_apps.iter().map(|e| to_entry(e, false)))
            .collect();
        entries.sort_by(|a, b| {
            a.display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase())
        });
        entries
    }

    fn initials(name: &str) -> String {
        let mut chars = name.chars().filter(|c| c.is_alphabetic());
        let first = chars.next().map(|c| c.to_ascii_uppercase()).unwrap_or('?');
        match chars.next().map(|c| c.to_ascii_uppercase()) {
            Some(s) => format!("{}{}", first, s),
            None => format!("{}", first),
        }
    }
}

impl Widget<UIDataAdapter> for AppsListWidget {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut UIDataAdapter, _env: &Env) {
        if let Event::MouseDown(mouse) = event {
            // Check badge option clicks first (badge_rects has 2 entries per row: [vi, en])
            let entries = Self::build_entries(data);
            for i in 0..entries.len() {
                let vi_rect = self.badge_rects.get(i * 2);
                let en_rect = self.badge_rects.get(i * 2 + 1);
                let clicked_vi = vi_rect.map_or(false, |r| r.contains(mouse.pos));
                let clicked_en = en_rect.map_or(false, |r| r.contains(mouse.pos));
                if clicked_vi || clicked_en {
                    let entry = &entries[i];
                    let want_vn = clicked_vi;
                    let already_correct = entry.is_vn == want_vn;
                    if !already_correct {
                        ctx.submit_command(
                            TOGGLE_APP_MODE
                                .with(entry.full_name.clone())
                                .to(druid::Target::Global),
                        );
                    }
                    return;
                }
            }
            // Row selection
            for (i, rect) in self.row_rects.iter().enumerate() {
                if rect.contains(mouse.pos) {
                    data.selected_app_index = i as i32;
                    ctx.request_paint();
                    break;
                }
            }
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &UIDataAdapter,
        _env: &Env,
    ) {
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &UIDataAdapter,
        data: &UIDataAdapter,
        _env: &Env,
    ) {
        if old_data.vn_apps != data.vn_apps || old_data.en_apps != data.en_apps {
            ctx.request_layout();
        } else if old_data.selected_app_index != data.selected_app_index {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &UIDataAdapter,
        _env: &Env,
    ) -> Size {
        let entries = Self::build_entries(data);
        let w = bc.max().width;
        // Segmented VI | EN toggle: each option ~28px wide, 22px tall, 2px gap, right-padded 14px
        let opt_w = 28.0_f64;
        let bh = 22.0_f64;
        let gap = 2.0_f64;
        self.row_rects = entries
            .iter()
            .enumerate()
            .map(|(i, _)| Rect::new(0.0, i as f64 * ROW_HEIGHT, w, (i + 1) as f64 * ROW_HEIGHT))
            .collect();
        self.badge_rects = entries
            .iter()
            .enumerate()
            .flat_map(|(i, _)| {
                let toggle_w = opt_w * 2.0 + gap;
                let toggle_x = w - toggle_w - 14.0;
                let toggle_y = i as f64 * ROW_HEIGHT + (ROW_HEIGHT - bh) / 2.0;
                [
                    Rect::new(toggle_x, toggle_y, toggle_x + opt_w, toggle_y + bh),
                    Rect::new(
                        toggle_x + opt_w + gap,
                        toggle_y,
                        toggle_x + toggle_w,
                        toggle_y + bh,
                    ),
                ]
            })
            .collect();
        Size::new(w, (entries.len() as f64 * ROW_HEIGHT).max(0.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &UIDataAdapter, _env: &Env) {
        let entries = Self::build_entries(data);
        let size = ctx.size();

        for (i, entry) in entries.iter().enumerate() {
            let rect = self.row_rects[i];
            let is_selected = data.selected_app_index == i as i32;

            if is_selected {
                ctx.fill(
                    RoundedRect::new(rect.x0, rect.y0, rect.x1, rect.y1, 0.0),
                    &Color::rgba8(0, 0, 0, 8),
                );
            }

            if i > 0 {
                ctx.fill(
                    Rect::new(54.0, rect.y0, size.width - 14.0, rect.y0 + 0.5),
                    &DIVIDER,
                );
            }

            // Avatar
            let avatar_x = 14.0;
            let avatar_y = rect.y0 + (ROW_HEIGHT - 36.0) / 2.0;
            let avatar_rect =
                RoundedRect::new(avatar_x, avatar_y, avatar_x + 36.0, avatar_y + 36.0, 8.0);
            ctx.fill(
                avatar_rect,
                &self.avatar_colors[i % self.avatar_colors.len()],
            );

            let initials = Self::initials(&entry.display_name);
            let init_layout = ctx
                .text()
                .new_text_layout(initials)
                .font(FontFamily::SYSTEM_UI, 13.0)
                .text_color(Color::WHITE)
                .build()
                .unwrap();
            ctx.draw_text(
                &init_layout,
                (
                    avatar_x + (36.0 - init_layout.size().width) / 2.0,
                    avatar_y + (36.0 - init_layout.size().height) / 2.0,
                ),
            );

            // App name
            let name_layout = ctx
                .text()
                .new_text_layout(entry.display_name.clone())
                .font(FontFamily::SYSTEM_UI, 14.0)
                .text_color(TEXT_PRIMARY)
                .build()
                .unwrap();
            ctx.draw_text(
                &name_layout,
                (
                    60.0,
                    rect.y0 + (ROW_HEIGHT - name_layout.size().height) / 2.0,
                ),
            );

            // Segmented VI | EN toggle
            let opt_w = 28.0_f64;
            let bh = 22.0_f64;
            let gap = 2.0_f64;
            let toggle_w = opt_w * 2.0 + gap;
            let toggle_x = size.width - toggle_w - 14.0;
            let toggle_y = rect.y0 + (ROW_HEIGHT - bh) / 2.0;

            for (j, (label, is_active, active_bg, active_border)) in [
                ("VI", entry.is_vn, BADGE_VI_BG, BADGE_VI_BORDER),
                ("EN", !entry.is_vn, BADGE_EN_BG, BADGE_EN_BORDER),
            ]
            .iter()
            .enumerate()
            {
                let opt_x = toggle_x + j as f64 * (opt_w + gap);
                let corners = if j == 0 {
                    [5.0, 0.0, 0.0, 5.0]
                } else {
                    [0.0, 5.0, 5.0, 0.0]
                };
                let opt_rr = druid::kurbo::RoundedRectRadii::new(
                    corners[0], corners[1], corners[2], corners[3],
                );
                let opt_rect = RoundedRect::from_rect(
                    Rect::new(opt_x, toggle_y, opt_x + opt_w, toggle_y + bh),
                    opt_rr,
                );

                if *is_active {
                    ctx.fill(opt_rect, active_bg);
                    ctx.stroke(opt_rect, active_border, 1.0);
                } else {
                    ctx.fill(opt_rect, &Color::rgba8(0, 0, 0, 0));
                    ctx.stroke(opt_rect, &Color::rgb8(210, 210, 210), 1.0);
                }

                let text_color = if *is_active {
                    *active_border
                } else {
                    Color::rgb8(170, 170, 170)
                };
                let opt_layout = ctx
                    .text()
                    .new_text_layout(*label)
                    .font(FontFamily::SYSTEM_UI, 11.0)
                    .text_color(text_color)
                    .build()
                    .unwrap();
                ctx.draw_text(
                    &opt_layout,
                    (
                        opt_x + (opt_w - opt_layout.size().width) / 2.0,
                        toggle_y + (bh - opt_layout.size().height) / 2.0,
                    ),
                );
            }
        }
    }
}
