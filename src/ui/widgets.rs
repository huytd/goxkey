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
        DIVIDER, GREEN, GREEN_BG, TEXT_PRIMARY,
    },
    data::UIDataAdapter,
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

            // Radio dot
            let dot_cx = rect.x0 + 14.0;
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
            let text_x = rect.x0 + (rect.width() - layout.size().width + 14.0) / 2.0 + 7.0;
            let text_y = rect.y0 + (rect.height() - layout.size().height) / 2.0;
            ctx.draw_text(&layout, (text_x, text_y));
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

    fn draw_icon_shortcuts(ctx: &mut PaintCtx, cx: f64, cy: f64, color: &Color) {
        ctx.stroke(
            RoundedRect::new(cx - 8.0, cy - 5.0, cx + 8.0, cy + 5.0, 2.0),
            color,
            1.5,
        );
        ctx.fill(
            RoundedRect::new(cx - 6.0, cy - 1.5, cx - 2.0, cy + 1.5, 1.0),
            color,
        );
        ctx.fill(
            RoundedRect::new(cx + 2.0, cy - 1.5, cx + 6.0, cy + 1.5, 1.0),
            color,
        );
    }

    fn draw_icon_advanced(ctx: &mut PaintCtx, cx: f64, cy: f64, color: &Color) {
        ctx.stroke(Circle::new((cx, cy), 7.0), color, 1.5);
        let mut hand = BezPath::new();
        hand.move_to((cx, cy));
        hand.line_to((cx, cy - 4.0));
        ctx.stroke(hand, color, 1.5);
        let mut hand2 = BezPath::new();
        hand2.move_to((cx, cy));
        hand2.line_to((cx + 3.0, cy + 2.0));
        ctx.stroke(hand2, color, 1.5);
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
        let tab_w = w / 4.0;
        self.tab_rects = (0..4)
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

        let labels = ["General", "Apps", "Shortcuts", "Advanced"];
        let icon_fns: [fn(&mut PaintCtx, f64, f64, &Color); 4] = [
            TabBar::draw_icon_general,
            TabBar::draw_icon_apps,
            TabBar::draw_icon_shortcuts,
            TabBar::draw_icon_advanced,
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
// AppsListWidget
// ══════════════════════════════════════════════════════════════════════════════

pub(super) struct CombinedAppEntry {
    pub(super) display_name: String,
    pub(super) full_name: String,
    pub(super) is_vn: bool,
}

pub(super) struct AppsListWidget {
    row_rects: Vec<Rect>,
    avatar_colors: Vec<Color>,
}

const ROW_HEIGHT: f64 = 52.0;

impl AppsListWidget {
    pub(super) fn new() -> Self {
        Self {
            row_rects: Vec::new(),
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
        let mut entries: Vec<CombinedAppEntry> = data
            .vn_apps
            .iter()
            .map(|e| CombinedAppEntry {
                display_name: std::path::Path::new(&e.name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&e.name)
                    .to_string(),
                full_name: e.name.clone(),
                is_vn: true,
            })
            .collect();
        for e in data.en_apps.iter() {
            entries.push(CombinedAppEntry {
                display_name: std::path::Path::new(&e.name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&e.name)
                    .to_string(),
                full_name: e.name.clone(),
                is_vn: false,
            });
        }
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
        self.row_rects = entries
            .iter()
            .enumerate()
            .map(|(i, _)| Rect::new(0.0, i as f64 * ROW_HEIGHT, w, (i + 1) as f64 * ROW_HEIGHT))
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

            // Language badge
            let (badge_label, badge_bg, badge_border) = if entry.is_vn {
                ("VI", BADGE_VI_BG, BADGE_VI_BORDER)
            } else {
                ("EN", BADGE_EN_BG, BADGE_EN_BORDER)
            };
            let badge_layout = ctx
                .text()
                .new_text_layout(badge_label)
                .font(FontFamily::SYSTEM_UI, 11.0)
                .text_color(badge_border)
                .build()
                .unwrap();
            let bw = badge_layout.size().width + 14.0;
            let bh = 22.0;
            let badge_x = size.width - bw - 14.0;
            let badge_y = rect.y0 + (ROW_HEIGHT - bh) / 2.0;
            let badge_rr = RoundedRect::new(badge_x, badge_y, badge_x + bw, badge_y + bh, 5.0);
            ctx.fill(badge_rr, &badge_bg);
            ctx.stroke(badge_rr, &badge_border, 1.0);
            ctx.draw_text(
                &badge_layout,
                (
                    badge_x + (bw - badge_layout.size().width) / 2.0,
                    badge_y + (bh - badge_layout.size().height) / 2.0,
                ),
            );
        }
    }
}
