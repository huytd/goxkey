use crate::{
    input::TypingMethod,
    platform::{defer_open_app_file_picker, defer_open_text_file_picker, defer_save_text_file_picker},
    UI_EVENT_SINK,
};
use druid::{
    kurbo::RoundedRect,
    piet::{FontFamily, Text, TextLayout, TextLayoutBuilder},
    widget::{
        Button, Container, FillStrat, Flex, Image, Label, LineBreaking, List, Painter, Scroll,
        TextBox, ViewSwitcher,
    },
    Application, Color, ImageBuf, Rect, RenderContext, Screen, Target, Widget, WidgetExt,
};

use super::{
    locale::t,
    colors::{
        BADGE_EN_BG, BADGE_EN_BORDER, BADGE_VI_BG, BADGE_VI_BORDER, BTN_RESET_BG, BTN_RESET_BORDER,
        CARD_BG, CARD_BORDER, DIVIDER, GREEN, TEXT_PRIMARY, TEXT_SECONDARY, TEXT_SECTION, WIN_BG,
    },
    controllers::UIController,
    data::{MacroEntry, UIDataAdapter},
    selectors::{
        ADD_MACRO, DELETE_MACRO, DELETE_SELECTED_APP, DELETE_SELECTED_MACRO,
        EXPORT_MACROS_TO_FILE, LOAD_MACROS_FROM_FILE, SET_EN_APP_FROM_PICKER,
        SHOW_ADD_MACRO_DIALOG, SHOW_EDIT_SHORTCUT_DIALOG,
    },
    widgets::{
        AppsListWidget, HotkeyBadgesWidget, InfoTooltip, MacroListWidget, SegmentedControl,
        ShortcutCaptureWidget, StyledCheckbox, TabBar, ToggleSwitch,
    },
    WINDOW_HEIGHT, WINDOW_WIDTH,
};

// ── Layout helpers ─────────────────────────────────────────────────────────────

/// A simple left-aligned text painter.
fn text_label(text: &'static str, font_size: f64, color: Color, height: f64) -> impl Widget<UIDataAdapter> {
    Painter::new(move |ctx, _: &UIDataAdapter, _| {
        let layout = ctx
            .text()
            .new_text_layout(text)
            .font(FontFamily::SYSTEM_UI, font_size)
            .text_color(color.clone())
            .build()
            .unwrap();
        ctx.draw_text(&layout, (0.0, 0.0));
    })
    .fix_height(height)
    .expand_width()
}

/// Title text (13pt, primary color, 18px height).
fn title_label(text: &'static str) -> impl Widget<UIDataAdapter> {
    text_label(text, 13.0, TEXT_PRIMARY, 18.0)
}

/// Subtitle text (12pt, secondary color, 16px height).
fn subtitle_label(text: &'static str) -> impl Widget<UIDataAdapter> {
    text_label(text, 12.0, TEXT_SECONDARY, 16.0)
}

/// A title + subtitle column, used in settings rows and custom rows.
fn title_subtitle_column(title: &'static str, subtitle: &'static str) -> impl Widget<UIDataAdapter> {
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(title_label(title))
        .with_child(subtitle_label(subtitle))
}

/// A rounded-rect button with centered text.
fn centered_btn(
    text: &'static str,
    width: f64,
    height: f64,
    bg: Color,
    text_color: Color,
    border: Option<(Color, f64)>,
) -> impl Widget<UIDataAdapter> {
    Painter::new(move |ctx, _: &UIDataAdapter, _| {
        let size = ctx.size();
        let rr = RoundedRect::new(0.0, 0.0, size.width, size.height, 7.0);
        ctx.fill(rr, &bg);
        if let Some((ref bc, bw)) = border {
            ctx.stroke(rr, bc, bw);
        }
        let layout = ctx
            .text()
            .new_text_layout(text)
            .font(FontFamily::SYSTEM_UI, 13.0)
            .text_color(text_color.clone())
            .build()
            .unwrap();
        ctx.draw_text(
            &layout,
            (
                (size.width - layout.size().width) / 2.0,
                (size.height - layout.size().height) / 2.0,
            ),
        );
    })
    .fix_size(width, height)
}

/// A "+" or "−" icon button for list add/remove actions.
fn symbol_btn(symbol: &'static str) -> impl Widget<UIDataAdapter> {
    Painter::new(move |ctx, _: &UIDataAdapter, _| {
        let size = ctx.size();
        let layout = ctx
            .text()
            .new_text_layout(symbol)
            .font(FontFamily::SYSTEM_UI, 18.0)
            .text_color(TEXT_PRIMARY)
            .build()
            .unwrap();
        ctx.draw_text(
            &layout,
            (
                (size.width - layout.size().width) / 2.0,
                (size.height - layout.size().height) / 2.0,
            ),
        );
    })
    .fix_size(44.0, 44.0)
}

/// A full-width horizontal divider (0.5px).
fn h_divider() -> impl Widget<UIDataAdapter> {
    Painter::new(|ctx, _: &UIDataAdapter, _| {
        let w = ctx.size().width;
        ctx.fill(Rect::new(0.0, 0.0, w, 0.5), &DIVIDER);
    })
    .fix_height(0.5)
    .expand_width()
}

fn section_label(text: &'static str) -> impl Widget<UIDataAdapter> {
    Painter::new(move |ctx, _data: &UIDataAdapter, _env| {
        let layout = ctx
            .text()
            .new_text_layout(text.to_uppercase())
            .font(FontFamily::SYSTEM_UI, 11.0)
            .text_color(TEXT_SECTION)
            .build()
            .unwrap();
        let h = ctx.size().height;
        ctx.draw_text(&layout, (0.0, (h - layout.size().height) / 2.0));
    })
    .fix_height(18.0)
    .expand_width()
    .padding((0.0, 0.0, 0.0, 6.0))
}

fn card_divider() -> impl Widget<UIDataAdapter> {
    Painter::new(|ctx, _data: &UIDataAdapter, _env| {
        let w = ctx.size().width;
        ctx.fill(Rect::new(14.0, 0.0, w - 14.0, 0.5), &DIVIDER);
    })
    .fix_height(0.5)
    .expand_width()
}

fn settings_row<TW: Widget<UIDataAdapter> + 'static>(
    title: &'static str,
    subtitle: &'static str,
    trailing: TW,
) -> impl Widget<UIDataAdapter> {
    Flex::row()
        .with_flex_child(title_subtitle_column(title, subtitle), 1.0)
        .with_child(trailing)
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Center)
        .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
        .must_fill_main_axis(true)
        .expand_width()
        .padding((14.0, 10.0))
}

fn settings_card<TW: Widget<UIDataAdapter> + 'static>(inner: TW) -> impl Widget<UIDataAdapter> {
    Container::new(inner)
        .background(CARD_BG)
        .border(CARD_BORDER, 0.5)
        .rounded(10.0)
}

// ── Tab content ────────────────────────────────────────────────────────────────

fn general_tab() -> impl Widget<UIDataAdapter> {
    let input_mode_card = settings_card(
        Flex::column()
            .with_child(
                Flex::row()
                    .with_flex_child(
                        title_subtitle_column(
                            t("general.vietnamese_input"),
                            t("general.enable_vietnamese"),
                        ),
                        1.0,
                    )
                    .with_child(ToggleSwitch.lens(UIDataAdapter::is_enabled).on_click(
                        |_, data: &mut UIDataAdapter, _| {
                            data.toggle_vietnamese();
                        },
                    ))
                    .cross_axis_alignment(druid::widget::CrossAxisAlignment::Center)
                    .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
                    .must_fill_main_axis(true)
                    .expand_width()
                    .padding((14.0, 10.0)),
            )
            .with_child(card_divider())
            .with_child(
                Flex::column()
                    .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                    .with_child(title_label(t("general.input_method")))
                    .with_spacer(8.0)
                    .with_child(
                        SegmentedControl::new(vec![
                            ("Telex", TypingMethod::Telex),
                            ("VNI", TypingMethod::VNI),
                            ("Telex + VNI", TypingMethod::TelexVNI),
                        ])
                        .lens(UIDataAdapter::typing_method)
                        .expand_width(),
                    )
                    .expand_width()
                    .padding((14.0, 10.0)),
            ),
    );

    let w_literal_card = settings_card(settings_row(
        t("general.w_literal"),
        t("general.w_literal_desc"),
        ToggleSwitch.lens(UIDataAdapter::is_w_literal_enabled),
    ));

    let system_card = settings_card(settings_row(
        t("general.launch_at_login"),
        t("general.launch_at_login_desc"),
        StyledCheckbox.lens(UIDataAdapter::launch_on_login),
    ));

    let edit_shortcut_btn = Painter::new(|ctx, _: &UIDataAdapter, _| {
        let size = ctx.size();
        let cx = size.width / 2.0;
        let cy = size.height / 2.0;
        // Pencil icon drawn with BezPath
        // Body: a thin parallelogram rotated ~45°
        let mut pencil = druid::kurbo::BezPath::new();
        pencil.move_to((cx - 1.5, cy + 6.0)); // bottom-left tip base
        pencil.line_to((cx - 6.0, cy + 1.5)); // top-left
        pencil.line_to((cx + 1.5, cy - 6.0)); // top-right
        pencil.line_to((cx + 6.0, cy - 1.5)); // bottom-right
        pencil.close_path();
        ctx.fill(pencil, &TEXT_SECONDARY);

        // Eraser nib at top
        let mut nib = druid::kurbo::BezPath::new();
        nib.move_to((cx + 1.5, cy - 6.0));
        nib.line_to((cx + 6.0, cy - 1.5));
        nib.line_to((cx + 7.5, cy - 3.0));
        nib.line_to((cx + 3.0, cy - 7.5));
        nib.close_path();
        ctx.fill(nib, &TEXT_PRIMARY);

        // Tip point at bottom
        let mut tip = druid::kurbo::BezPath::new();
        tip.move_to((cx - 1.5, cy + 6.0));
        tip.line_to((cx - 6.0, cy + 1.5));
        tip.line_to((cx - 8.0, cy + 8.0));
        tip.close_path();
        ctx.fill(tip, &TEXT_SECONDARY);
    })
    .fix_size(24.0, 24.0)
    .on_click(|ctx, _: &mut UIDataAdapter, _| {
        ctx.submit_command(SHOW_EDIT_SHORTCUT_DIALOG.to(druid::Target::Global));
    });

    let shortcut_card = settings_card(
        Flex::row()
            .with_flex_child(
                title_subtitle_column(
                    t("general.toggle_shortcut"),
                    t("general.toggle_shortcut_desc"),
                ),
                1.0,
            )
            .with_child(HotkeyBadgesWidget::new())
            .with_spacer(8.0)
            .with_child(edit_shortcut_btn)
            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Center)
            .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
            .must_fill_main_axis(true)
            .expand_width()
            .padding((14.0, 10.0)),
    );

    let footer = Flex::row()
        .with_flex_spacer(1.0)
        .with_child(
            centered_btn(
                t("general.reset_defaults"),
                120.0, 30.0,
                BTN_RESET_BG,
                Color::rgb8(51, 51, 51),
                Some((BTN_RESET_BORDER, 0.5)),
            )
            .on_click(|ctx, _data: &mut UIDataAdapter, _env| {
                ctx.submit_command(super::selectors::RESET_DEFAULTS);
            }),
        )
        .with_spacer(8.0)
        .with_child(
            centered_btn(t("general.done"), 70.0, 30.0, GREEN, Color::WHITE, None)
                .on_click(|ctx, _data: &mut UIDataAdapter, _env| {
                    ctx.window().hide();
                }),
        )
        .expand_width();

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(section_label(t("general.input_mode")))
        .with_child(input_mode_card)
        .with_spacer(8.0)
        .with_child(w_literal_card)
        .with_spacer(20.0)
        .with_child(section_label(t("general.system")))
        .with_child(system_card)
        .with_spacer(20.0)
        .with_child(section_label(t("general.shortcut")))
        .with_child(shortcut_card)
        .with_flex_spacer(1.0)
        .with_child(footer)
        .padding((24.0, 20.0, 24.0, 24.0))
}

fn apps_tab() -> impl Widget<UIDataAdapter> {
    let description = title_label(t("apps.description"));

    let legend = Painter::new(|ctx, _: &UIDataAdapter, _| {
        let mut x = 0.0;
        let bh = 22.0;
        let badge_y = (26.0 - bh) / 2.0;

        let vi_layout = ctx
            .text()
            .new_text_layout("VI")
            .font(FontFamily::SYSTEM_UI, 11.0)
            .text_color(BADGE_VI_BORDER)
            .build()
            .unwrap();
        let bw = vi_layout.size().width + 14.0;
        let vi_rr = RoundedRect::new(x, badge_y, x + bw, badge_y + bh, 5.0);
        ctx.fill(vi_rr, &BADGE_VI_BG);
        ctx.stroke(vi_rr, &BADGE_VI_BORDER, 1.0);
        ctx.draw_text(
            &vi_layout,
            (
                x + (bw - vi_layout.size().width) / 2.0,
                badge_y + (bh - vi_layout.size().height) / 2.0,
            ),
        );
        x += bw + 8.0;

        let vn_label = ctx
            .text()
            .new_text_layout(t("apps.vietnamese"))
            .font(FontFamily::SYSTEM_UI, 13.0)
            .text_color(TEXT_PRIMARY)
            .build()
            .unwrap();
        ctx.draw_text(&vn_label, (x, (26.0 - vn_label.size().height) / 2.0));
        x += vn_label.size().width + 20.0;

        let en_layout = ctx
            .text()
            .new_text_layout("EN")
            .font(FontFamily::SYSTEM_UI, 11.0)
            .text_color(BADGE_EN_BORDER)
            .build()
            .unwrap();
        let bw_en = en_layout.size().width + 14.0;
        let en_rr = RoundedRect::new(x, badge_y, x + bw_en, badge_y + bh, 5.0);
        ctx.fill(en_rr, &BADGE_EN_BG);
        ctx.stroke(en_rr, &BADGE_EN_BORDER, 1.0);
        ctx.draw_text(
            &en_layout,
            (
                x + (bw_en - en_layout.size().width) / 2.0,
                badge_y + (bh - en_layout.size().height) / 2.0,
            ),
        );
        x += bw_en + 8.0;

        let en_label = ctx
            .text()
            .new_text_layout(t("apps.english"))
            .font(FontFamily::SYSTEM_UI, 13.0)
            .text_color(TEXT_PRIMARY)
            .build()
            .unwrap();
        ctx.draw_text(&en_label, (x, (26.0 - en_label.size().height) / 2.0));
    })
    .fix_height(26.0)
    .expand_width();

    let app_list = {
        let mut scroll = Scroll::new(AppsListWidget::new().expand_width());
        scroll.set_enabled_scrollbars(druid::scroll_component::ScrollbarsEnabled::Vertical);
        scroll.set_horizontal_scroll_enabled(false);
        scroll
    };

    let add_btn = symbol_btn("+").on_click(|_, _, _| {
        defer_open_app_file_picker(Box::new(|name| {
            if let Some(name) = name {
                if let Some(sink) = UI_EVENT_SINK.get() {
                    let _ = sink.submit_command(SET_EN_APP_FROM_PICKER, name, Target::Auto);
                }
            }
        }));
    });

    let remove_btn = Painter::new(|ctx, data: &UIDataAdapter, _| {
        let size = ctx.size();
        let is_enabled = data.selected_app_index >= 0;
        let color = if is_enabled {
            TEXT_PRIMARY
        } else {
            Color::rgb8(187, 187, 187)
        };
        ctx.fill(Rect::new(0.0, 10.0, 0.5, size.height - 10.0), &DIVIDER);
        let layout = ctx
            .text()
            .new_text_layout("−")
            .font(FontFamily::SYSTEM_UI, 18.0)
            .text_color(color)
            .build()
            .unwrap();
        ctx.draw_text(
            &layout,
            (
                (size.width - layout.size().width) / 2.0 + 0.5,
                (size.height - layout.size().height) / 2.0,
            ),
        );
    })
    .fix_size(44.0, 44.0)
    .on_click(|ctx, data: &mut UIDataAdapter, _| {
        if data.selected_app_index >= 0 {
            ctx.submit_command(DELETE_SELECTED_APP.to(Target::Global));
        }
    });

    let card = Container::new(
        Flex::column()
            .with_flex_child(app_list.expand(), 1.0)
            .with_child(h_divider())
            .with_child(
                Flex::row()
                    .with_child(add_btn)
                    .with_child(remove_btn)
                    .with_flex_spacer(1.0)
                    .expand_width(),
            ),
    )
    .background(CARD_BG)
    .border(CARD_BORDER, 0.5)
    .rounded(10.0);

    let per_app_toggle_card = settings_card(settings_row(
        t("apps.per_app_toggle"),
        t("apps.per_app_toggle_desc"),
        ToggleSwitch.lens(UIDataAdapter::is_auto_toggle_enabled),
    ));

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(description)
        .with_spacer(12.0)
        .with_child(per_app_toggle_card)
        .with_spacer(16.0)
        .with_child(legend)
        .with_spacer(12.0)
        .with_flex_child(card.expand_height(), 1.0)
        .must_fill_main_axis(true)
        .expand()
        .padding((24.0, 20.0, 24.0, 24.0))
}

fn advanced_tab() -> impl Widget<UIDataAdapter> {
    let description = title_label(t("macro.description"));

    let enable_row = settings_card(settings_row(
        t("macro.text_expansion"),
        t("macro.enable"),
        ToggleSwitch.lens(UIDataAdapter::is_macro_enabled),
    ));

    let autocap_row = settings_card(
        Flex::row()
            .with_flex_child(
                title_subtitle_column(t("macro.auto_capitalize"), t("macro.auto_capitalize_desc")),
                1.0,
            )
            .with_child(
                InfoTooltip::new("ko → không\nKo → Không\nKO → KHÔNG")
                    .padding((0.0, 0.0, 8.0, 0.0)),
            )
            .with_child(StyledCheckbox.lens(UIDataAdapter::is_macro_autocap_enabled))
            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Center)
            .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
            .must_fill_main_axis(true)
            .expand_width()
            .padding((14.0, 10.0)),
    );

    let macro_list = {
        let mut scroll = Scroll::new(MacroListWidget::new().expand_width());
        scroll.set_enabled_scrollbars(druid::scroll_component::ScrollbarsEnabled::Vertical);
        scroll.set_horizontal_scroll_enabled(false);
        scroll
    };

    let add_btn = symbol_btn("+").on_click(|ctx, _data: &mut UIDataAdapter, _| {
        ctx.submit_command(SHOW_ADD_MACRO_DIALOG.to(Target::Global));
    });

    let remove_btn = Painter::new(|ctx, data: &UIDataAdapter, _| {
        let size = ctx.size();
        let is_enabled = data.selected_macro_index >= 0;
        let color = if is_enabled {
            TEXT_PRIMARY
        } else {
            Color::rgb8(187, 187, 187)
        };
        ctx.fill(Rect::new(0.0, 10.0, 0.5, size.height - 10.0), &DIVIDER);
        let layout = ctx
            .text()
            .new_text_layout("−")
            .font(FontFamily::SYSTEM_UI, 18.0)
            .text_color(color)
            .build()
            .unwrap();
        ctx.draw_text(
            &layout,
            (
                (size.width - layout.size().width) / 2.0 + 0.5,
                (size.height - layout.size().height) / 2.0,
            ),
        );
    })
    .fix_size(44.0, 44.0)
    .on_click(|ctx, data: &mut UIDataAdapter, _| {
        if data.selected_macro_index >= 0 {
            ctx.submit_command(DELETE_SELECTED_MACRO.to(Target::Global));
        }
    });

    let load_btn = Painter::new(|ctx, _: &UIDataAdapter, _| {
        let size = ctx.size();
        ctx.fill(Rect::new(size.width - 0.5, 10.0, size.width, size.height - 10.0), &DIVIDER);
        let layout = ctx
            .text()
            .new_text_layout(t("macro.load"))
            .font(FontFamily::SYSTEM_UI, 12.0)
            .text_color(TEXT_PRIMARY)
            .build()
            .unwrap();
        ctx.draw_text(
            &layout,
            (
                (size.width - layout.size().width) / 2.0,
                (size.height - layout.size().height) / 2.0,
            ),
        );
    })
    .fix_size(60.0, 44.0)
    .on_click(|ctx, _data: &mut UIDataAdapter, _| {
        ctx.submit_command(LOAD_MACROS_FROM_FILE.to(Target::Global));
    });

    let export_btn = Painter::new(|ctx, _: &UIDataAdapter, _| {
        let size = ctx.size();
        let layout = ctx
            .text()
            .new_text_layout(t("macro.export"))
            .font(FontFamily::SYSTEM_UI, 12.0)
            .text_color(TEXT_PRIMARY)
            .build()
            .unwrap();
        ctx.draw_text(
            &layout,
            (
                (size.width - layout.size().width) / 2.0,
                (size.height - layout.size().height) / 2.0,
            ),
        );
    })
    .fix_size(60.0, 44.0)
    .on_click(|ctx, _data: &mut UIDataAdapter, _| {
        ctx.submit_command(EXPORT_MACROS_TO_FILE.to(Target::Global));
    });

    let card = Container::new(
        Flex::column()
            .with_flex_child(macro_list.expand(), 1.0)
            .with_child(h_divider())
            .with_child(
                Flex::row()
                    .with_child(add_btn)
                    .with_child(remove_btn)
                    .with_flex_spacer(1.0)
                    .with_child(
                        Painter::new(|ctx, _: &UIDataAdapter, _| {
                            let h = ctx.size().height;
                            ctx.fill(Rect::new(0.0, 10.0, 0.5, h - 10.0), &DIVIDER);
                        })
                        .fix_size(0.5, 44.0),
                    )
                    .with_child(load_btn)
                    .with_child(export_btn)
                    .expand_width(),
            ),
    )
    .background(CARD_BG)
    .border(CARD_BORDER, 0.5)
    .rounded(10.0);

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(description)
        .with_spacer(12.0)
        .with_child(enable_row)
        .with_spacer(8.0)
        .with_child(autocap_row)
        .with_spacer(16.0)
        .with_flex_child(card.expand_height(), 1.0)
        .must_fill_main_axis(true)
        .expand()
        .padding((24.0, 20.0, 24.0, 24.0))
}

// ── List row helpers ───────────────────────────────────────────────────────────

fn macro_row_item() -> impl Widget<MacroEntry> {
    Flex::row()
        .with_flex_child(
            Label::dynamic(|e: &MacroEntry, _| e.from.clone())
                .with_line_break_mode(LineBreaking::WordWrap)
                .align_left(),
            2.0,
        )
        .with_flex_child(
            Label::dynamic(|e: &MacroEntry, _| e.to.clone())
                .with_line_break_mode(LineBreaking::WordWrap)
                .align_left(),
            2.0,
        )
        .with_flex_child(
            Button::new("×").on_click(|ctx, data: &mut MacroEntry, _| {
                ctx.submit_command(DELETE_MACRO.with(data.from.clone()).to(Target::Global))
            }),
            1.0,
        )
        .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Baseline)
        .expand_width()
        .border(Color::rgb8(224, 224, 224), 0.5)
}

// ── Public UI builders ─────────────────────────────────────────────────────────

pub fn main_ui_builder() -> impl Widget<UIDataAdapter> {
    Flex::column()
        .with_child(
            TabBar::new()
                .lens(UIDataAdapter::active_tab)
                .fix_height(58.0),
        )
        .with_flex_child(
            ViewSwitcher::new(
                |data: &UIDataAdapter, _env| data.active_tab,
                |tab, _data, _env| match tab {
                    1 => Box::new(apps_tab()),
                    2 => Box::new(advanced_tab()),
                    _ => Box::new(general_tab()),
                },
            )
            .expand(),
            1.0,
        )
        .background(WIN_BG)
        .controller(UIController)
}

pub fn permission_request_ui_builder() -> impl Widget<()> {
    use super::colors::{CARD_BORDER, GREEN, TEXT_PRIMARY, TEXT_SECONDARY, WIN_BG};
    let image_data = ImageBuf::from_data(include_bytes!("../../assets/accessibility.png")).unwrap();

    let title_label = Label::new(t("perm.title"))
        .with_text_color(TEXT_PRIMARY)
        .with_font(druid::FontDescriptor::new(FontFamily::SYSTEM_UI).with_size(13.0))
        .with_line_break_mode(LineBreaking::WordWrap);

    let img_container = Container::new(
        Image::new(image_data).fill_mode(FillStrat::Cover)
    )
    .rounded(8.0)
    .border(CARD_BORDER, 1.0);

    let subtitle_label = Label::new(t("perm.subtitle"))
        .with_text_color(TEXT_SECONDARY)
        .with_font(druid::FontDescriptor::new(FontFamily::SYSTEM_UI).with_size(12.0))
        .with_line_break_mode(LineBreaking::WordWrap);

    let exit_btn = Painter::new(|ctx, _: &(), _| {
        let size = ctx.size();
        let rr = RoundedRect::new(0.0, 0.0, size.width, size.height, 7.0);
        ctx.fill(rr, &GREEN);
        let layout = ctx
            .text()
            .new_text_layout(t("perm.exit"))
            .font(FontFamily::SYSTEM_UI, 13.0)
            .text_color(Color::WHITE)
            .build()
            .unwrap();
        ctx.draw_text(
            &layout,
            (
                (size.width - layout.size().width) / 2.0,
                (size.height - layout.size().height) / 2.0,
            ),
        );
    })
    .fix_size(90.0, 30.0)
    .on_click(|_, _, _| Application::global().quit());

    let buttons = Flex::row()
        .with_flex_spacer(1.0)
        .with_child(exit_btn)
        .expand_width();

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .main_axis_alignment(druid::widget::MainAxisAlignment::Start)
        .with_child(title_label)
        .with_spacer(16.0)
        .with_child(img_container)
        .with_spacer(16.0)
        .with_child(subtitle_label)
        .with_flex_spacer(1.0)
        .with_child(buttons)
        .padding((24.0, 20.0, 24.0, 20.0))
        .background(WIN_BG)
}

pub fn macro_editor_ui_builder() -> impl Widget<UIDataAdapter> {
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_flex_child(
            {
                let mut scroll = Scroll::new(
                    List::new(macro_row_item)
                        .lens(UIDataAdapter::macro_table)
                        .expand_width(),
                );
                scroll.set_enabled_scrollbars(druid::scroll_component::ScrollbarsEnabled::Vertical);
                scroll.set_horizontal_scroll_enabled(false);
                scroll
            }
            .expand(),
            1.0,
        )
        .with_child(
            Flex::row()
                .with_flex_child(
                    TextBox::new()
                        .with_placeholder(t("macro.shorthand"))
                        .expand_width()
                        .lens(UIDataAdapter::new_macro_from),
                    2.0,
                )
                .with_flex_child(
                    TextBox::new()
                        .with_placeholder(t("macro.replacement"))
                        .expand_width()
                        .lens(UIDataAdapter::new_macro_to),
                    2.0,
                )
                .with_child(
                    Button::new(t("button.add"))
                        .on_click(|ctx, _, _| ctx.submit_command(ADD_MACRO.to(Target::Global))),
                )
                .expand_width(),
        )
        .padding(8.0)
}

pub fn center_window_position() -> (f64, f64) {
    let screen_rect = Screen::get_display_rect();
    let x = (screen_rect.width() - WINDOW_WIDTH) / 2.0;
    let y = (screen_rect.height() - WINDOW_HEIGHT) / 2.0;
    (x, y)
}

// ── Add Macro Dialog ───────────────────────────────────────────────────────────

pub const ADD_MACRO_DIALOG_WIDTH: f64 = 340.0;
pub const ADD_MACRO_DIALOG_HEIGHT: f64 = 208.0;

// ── Edit Shortcut Dialog ───────────────────────────────────────────────────────

pub const EDIT_SHORTCUT_DIALOG_WIDTH: f64 = 340.0;
pub const EDIT_SHORTCUT_DIALOG_HEIGHT: f64 = 200.0;

fn styled_text_input(placeholder: &'static str) -> impl Widget<String> {
    use druid::theme;
    TextBox::new()
        .with_placeholder(placeholder)
        .expand_width()
        .fix_height(32.0)
        .env_scope(|env, _| {
            env.set(theme::BACKGROUND_LIGHT, Color::WHITE);
            env.set(theme::BACKGROUND_DARK, Color::WHITE);
            env.set(theme::TEXTBOX_BORDER_WIDTH, 0.0);
            env.set(theme::TEXTBOX_BORDER_RADIUS, 8.0);
            env.set(theme::TEXT_COLOR, Color::rgb8(17, 17, 17));
            env.set(theme::CURSOR_COLOR, Color::rgb8(17, 17, 17));
            env.set(
                theme::TEXTBOX_INSETS,
                druid::Insets::new(6.0, 6.0, 6.0, 3.0),
            );
        })
}

pub fn add_macro_dialog_ui_builder() -> impl Widget<UIDataAdapter> {
    use super::colors::{BTN_RESET_BG, BTN_RESET_BORDER, GREEN};

    let shorthand_label = subtitle_label(t("macro.shorthand"));
    let replacement_label = subtitle_label(t("macro.replacement"));

    let cancel_btn = centered_btn(
        t("button.cancel"),
        90.0, 30.0,
        BTN_RESET_BG,
        Color::rgb8(51, 51, 51),
        Some((BTN_RESET_BORDER, 0.5)),
    )
    .on_click(|ctx, data: &mut UIDataAdapter, _| {
        data.new_macro_from = String::new();
        data.new_macro_to = String::new();
        ctx.window().close();
    });

    let add_btn = Painter::new(|ctx, data: &UIDataAdapter, _| {
        let size = ctx.size();
        let rr = RoundedRect::new(0.0, 0.0, size.width, size.height, 7.0);
        let enabled = !data.new_macro_from.is_empty() && !data.new_macro_to.is_empty();
        let bg = if enabled {
            GREEN
        } else {
            Color::rgb8(150, 150, 150)
        };
        ctx.fill(rr, &bg);
        let layout = ctx
            .text()
            .new_text_layout(t("button.add"))
            .font(FontFamily::SYSTEM_UI, 13.0)
            .text_color(Color::WHITE)
            .build()
            .unwrap();
        ctx.draw_text(
            &layout,
            (
                (size.width - layout.size().width) / 2.0,
                (size.height - layout.size().height) / 2.0,
            ),
        );
    })
    .fix_size(70.0, 30.0)
    .on_click(|ctx, data: &mut UIDataAdapter, _| {
        if !data.new_macro_from.is_empty() && !data.new_macro_to.is_empty() {
            ctx.submit_command(ADD_MACRO.to(Target::Global));
            ctx.window().close();
        }
    });

    let buttons = Flex::row()
        .with_flex_spacer(1.0)
        .with_child(cancel_btn)
        .with_spacer(8.0)
        .with_child(add_btn)
        .expand_width();

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(shorthand_label)
        .with_spacer(4.0)
        .with_child(
            Container::new(styled_text_input("nope").lens(UIDataAdapter::new_macro_from))
                .background(Color::WHITE)
                .border(Color::rgb8(204, 204, 204), 1.0)
                .rounded(8.0)
                .expand_width(),
        )
        .with_spacer(12.0)
        .with_child(replacement_label)
        .with_spacer(4.0)
        .with_child(
            Container::new(styled_text_input("dạ, thưa sếp").lens(UIDataAdapter::new_macro_to))
                .background(Color::WHITE)
                .border(Color::rgb8(204, 204, 204), 1.0)
                .rounded(8.0)
                .expand_width(),
        )
        .with_flex_spacer(1.0)
        .with_child(buttons)
        .padding((24.0, 20.0, 24.0, 20.0))
        .background(WIN_BG)
        .expand()
}

pub fn edit_shortcut_dialog_ui_builder() -> impl Widget<UIDataAdapter> {
    use super::{colors::TEXT_SECONDARY, selectors::SAVE_SHORTCUT};

    let title_label = subtitle_label(t("shortcut.new"));
    let hint_label = text_label(t("shortcut.hint"), 11.0, TEXT_SECONDARY, 14.0);

    let cancel_btn = centered_btn(
        t("button.cancel"),
        90.0, 30.0,
        BTN_RESET_BG,
        Color::rgb8(51, 51, 51),
        Some((BTN_RESET_BORDER, 0.5)),
    )
    .on_click(|ctx, _: &mut UIDataAdapter, _| {
        ctx.window().close();
    });

    let save_btn = Painter::new(|ctx, data: &UIDataAdapter, _| {
        let size = ctx.size();
        let rr = druid::kurbo::RoundedRect::new(0.0, 0.0, size.width, size.height, 7.0);
        let enabled = !data.pending_shortcut_display.is_empty();
        let bg = if enabled {
            GREEN
        } else {
            Color::rgb8(150, 150, 150)
        };
        ctx.fill(rr, &bg);
        let layout = ctx
            .text()
            .new_text_layout(t("button.save"))
            .font(FontFamily::SYSTEM_UI, 13.0)
            .text_color(Color::WHITE)
            .build()
            .unwrap();
        ctx.draw_text(
            &layout,
            (
                (size.width - layout.size().width) / 2.0,
                (size.height - layout.size().height) / 2.0,
            ),
        );
    })
    .fix_size(70.0, 30.0)
    .on_click(|ctx, data: &mut UIDataAdapter, _| {
        if !data.pending_shortcut_display.is_empty() {
            ctx.submit_command(
                SAVE_SHORTCUT
                    .with((
                        data.pending_shortcut_super,
                        data.pending_shortcut_ctrl,
                        data.pending_shortcut_alt,
                        data.pending_shortcut_shift,
                        data.pending_shortcut_letter.clone(),
                    ))
                    .to(Target::Global),
            );
            ctx.window().close();
        }
    });

    let buttons = Flex::row()
        .with_flex_spacer(1.0)
        .with_child(cancel_btn)
        .with_spacer(8.0)
        .with_child(save_btn)
        .expand_width();

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(title_label)
        .with_spacer(4.0)
        .with_child(hint_label)
        .with_spacer(16.0)
        .with_child(
            Container::new(ShortcutCaptureWidget::new())
                .background(Color::WHITE)
                .border(Color::rgb8(204, 204, 204), 1.0)
                .rounded(8.0)
                .fix_height(52.0)
                .expand_width(),
        )
        .with_flex_spacer(1.0)
        .with_child(buttons)
        .padding((24.0, 20.0, 24.0, 20.0))
        .background(WIN_BG)
        .expand()
}
