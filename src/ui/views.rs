use crate::{input::TypingMethod, platform::defer_open_app_file_picker, UI_EVENT_SINK};
use druid::{
    kurbo::RoundedRect,
    piet::{FontFamily, Text, TextLayout, TextLayoutBuilder},
    widget::{
        Button, Container, EnvScope, FillStrat, Flex, Image, Label, LineBreaking, List, Painter,
        Scroll, TextBox, ViewSwitcher,
    },
    Application, Color, ImageBuf, Rect, RenderContext, Screen, Target, Widget, WidgetExt,
};

use super::{
    colors::{
        theme_from_env, Theme, BADGE_EN_BG, BADGE_EN_BORDER, BADGE_VI_BG, BADGE_VI_BORDER, GREEN,
        IS_DARK,
    },
    controllers::UIController,
    data::{MacroEntry, UIDataAdapter},
    locale::t,
    selectors::{
        ADD_MACRO, DELETE_MACRO, DELETE_SELECTED_APP, DELETE_SELECTED_MACRO, EXPORT_MACROS_TO_FILE,
        LOAD_MACROS_FROM_FILE, SET_EN_APP_FROM_PICKER, SHOW_ADD_MACRO_DIALOG,
        SHOW_EDIT_SHORTCUT_DIALOG,
    },
    widgets::{
        AppsListWidget, HotkeyBadgesWidget, InfoTooltip, MacroListWidget, SegmentedControl,
        ShortcutCaptureWidget, StyledCheckbox, TabBar, ToggleSwitch, U32SegmentedControl,
    },
    WINDOW_HEIGHT, WINDOW_WIDTH,
};

fn text_label(
    key: &'static str,
    font_size: f64,
    color_fn: fn(&Theme) -> Color,
    height: f64,
) -> impl Widget<UIDataAdapter> {
    Painter::new(move |ctx, _: &UIDataAdapter, env| {
        let theme = theme_from_env(env);
        let color = color_fn(&theme);
        let layout = make_text_layout(ctx, t(key), font_size, &color);
        ctx.draw_text(&layout, (0.0, 0.0));
    })
    .fix_height(height)
    .expand_width()
}

fn title_label(key: &'static str) -> impl Widget<UIDataAdapter> {
    text_label(key, 13.0, |t| t.text_primary, 18.0)
}

fn subtitle_label(key: &'static str) -> impl Widget<UIDataAdapter> {
    text_label(key, 12.0, |t| t.text_secondary, 16.0)
}

fn title_subtitle_column(
    title_key: &'static str,
    subtitle_key: &'static str,
) -> impl Widget<UIDataAdapter> {
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(title_label(title_key))
        .with_child(subtitle_label(subtitle_key))
}

fn centered_btn(
    key: &'static str,
    width: f64,
    height: f64,
    bg_fn: fn(&Theme) -> Color,
    text_color_fn: fn(&Theme) -> Color,
    border_fn: Option<fn(&Theme) -> Color>,
) -> impl Widget<UIDataAdapter> {
    Painter::new(move |ctx, _: &UIDataAdapter, env| {
        let theme = theme_from_env(env);
        let size = ctx.size();
        let rr = RoundedRect::new(0.0, 0.0, size.width, size.height, 7.0);
        ctx.fill(rr, &bg_fn(&theme));
        if let Some(border_f) = border_fn {
            ctx.stroke(rr, &border_f(&theme), 0.5);
        }
        draw_centered_text(ctx, t(key), 13.0, &text_color_fn(&theme));
    })
    .fix_size(width, height)
}

fn make_text_layout(
    ctx: &mut druid::PaintCtx,
    text: &str,
    font_size: f64,
    color: &Color,
) -> druid::piet::PietTextLayout {
    ctx.text()
        .new_text_layout(text.to_owned())
        .font(FontFamily::SYSTEM_UI, font_size)
        .text_color(color.clone())
        .build()
        .unwrap()
}

fn draw_centered_text(ctx: &mut druid::PaintCtx, text: &str, font_size: f64, color: &Color) {
    let layout = make_text_layout(ctx, text, font_size, color);
    let size = ctx.size();
    ctx.draw_text(
        &layout,
        (
            (size.width - layout.size().width) / 2.0,
            (size.height - layout.size().height) / 2.0,
        ),
    );
}

fn symbol_btn(symbol: &'static str) -> impl Widget<UIDataAdapter> {
    Painter::new(move |ctx, _: &UIDataAdapter, env| {
        let theme = theme_from_env(env);
        draw_centered_text(ctx, symbol, 18.0, &theme.text_primary);
    })
    .fix_size(44.0, 44.0)
}

fn remove_btn(is_enabled_fn: fn(&UIDataAdapter) -> bool) -> impl Widget<UIDataAdapter> {
    Painter::new(move |ctx, data: &UIDataAdapter, env| {
        let theme = theme_from_env(env);
        let size = ctx.size();
        let color = if is_enabled_fn(data) {
            theme.text_primary
        } else {
            Color::rgb8(187, 187, 187)
        };
        ctx.fill(
            Rect::new(0.0, 10.0, 0.5, size.height - 10.0),
            &theme.divider,
        );
        draw_centered_text(ctx, "−", 18.0, &color);
    })
    .fix_size(44.0, 44.0)
}

fn toolbar_btn(key: &'static str, divider: Option<&'static str>) -> impl Widget<UIDataAdapter> {
    Painter::new(move |ctx, _: &UIDataAdapter, env| {
        let theme = theme_from_env(env);
        let size = ctx.size();
        if let Some(side) = divider {
            let x = if side == "left" {
                0.0
            } else {
                size.width - 0.5
            };
            ctx.fill(
                Rect::new(x, 10.0, x + 0.5, size.height - 10.0),
                &theme.divider,
            );
        }
        draw_centered_text(ctx, t(key), 12.0, &theme.text_primary);
    })
    .fix_size(60.0, 44.0)
}

fn h_divider() -> impl Widget<UIDataAdapter> {
    Painter::new(|ctx, _: &UIDataAdapter, env| {
        let theme = theme_from_env(env);
        let w = ctx.size().width;
        ctx.fill(Rect::new(0.0, 0.0, w, 0.5), &theme.divider);
    })
    .fix_height(0.5)
    .expand_width()
}

fn section_label(key: &'static str) -> impl Widget<UIDataAdapter> {
    Painter::new(move |ctx, _data: &UIDataAdapter, env| {
        let theme = theme_from_env(env);
        let layout = make_text_layout(ctx, &t(key).to_uppercase(), 11.0, &theme.text_section);
        let h = ctx.size().height;
        ctx.draw_text(&layout, (0.0, (h - layout.size().height) / 2.0));
    })
    .fix_height(18.0)
    .expand_width()
    .padding((0.0, 0.0, 0.0, 6.0))
}

fn card_divider() -> impl Widget<UIDataAdapter> {
    Painter::new(|ctx, _data: &UIDataAdapter, env| {
        let theme = theme_from_env(env);
        let w = ctx.size().width;
        ctx.fill(Rect::new(14.0, 0.0, w - 14.0, 0.5), &theme.divider);
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
        .background(Painter::new(|ctx, _, env| {
            let theme = theme_from_env(env);
            let rect = ctx.size().to_rect();
            let rr = RoundedRect::from_rect(rect, 10.0);
            ctx.fill(rr, &theme.card_bg);
            ctx.stroke(rr, &theme.card_border, 0.5);
        }))
        .rounded(10.0)
}

fn tab_body() -> Flex<UIDataAdapter> {
    Flex::column().cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
}

const TAB_PADDING: (f64, f64, f64, f64) = (24.0, 20.0, 24.0, 24.0);

fn option_group<SW: Widget<UIDataAdapter> + 'static>(
    header: impl Widget<UIDataAdapter> + 'static,
    control: SW,
) -> impl Widget<UIDataAdapter> {
    settings_card(
        Flex::column()
            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
            .with_child(header)
            .with_spacer(8.0)
            .with_child(control.expand_width())
            .expand_width()
            .padding((14.0, 10.0)),
    )
}

fn v_scroll<W: Widget<UIDataAdapter> + 'static>(inner: W) -> Scroll<UIDataAdapter, W> {
    let mut scroll = Scroll::new(inner);
    scroll.set_enabled_scrollbars(druid::scroll_component::ScrollbarsEnabled::Vertical);
    scroll.set_horizontal_scroll_enabled(false);
    scroll
}

fn list_card(
    list: impl Widget<UIDataAdapter> + 'static,
    toolbar: impl Widget<UIDataAdapter> + 'static,
) -> impl Widget<UIDataAdapter> {
    Container::new(
        Flex::column()
            .with_flex_child(v_scroll(list.expand_width()).expand(), 1.0)
            .with_child(h_divider())
            .with_child(toolbar.expand_width()),
    )
    .background(Painter::new(|ctx, _, env| {
        let theme = theme_from_env(env);
        let rect = ctx.size().to_rect();
        let rr = RoundedRect::from_rect(rect, 10.0);
        ctx.fill(rr, &theme.card_bg);
        ctx.stroke(rr, &theme.card_border, 0.5);
    }))
    .rounded(10.0)
}

fn action_btn(
    key: &'static str,
    enabled_fn: fn(&UIDataAdapter) -> bool,
) -> impl Widget<UIDataAdapter> {
    Painter::new(move |ctx, data: &UIDataAdapter, _| {
        let size = ctx.size();
        let rr = RoundedRect::new(0.0, 0.0, size.width, size.height, 7.0);
        let bg = if enabled_fn(data) {
            GREEN
        } else {
            Color::rgb8(150, 150, 150)
        };
        ctx.fill(rr, &bg);
        draw_centered_text(ctx, t(key), 13.0, &Color::WHITE);
    })
    .fix_size(70.0, 30.0)
}

fn dialog_buttons(
    cancel: impl Widget<UIDataAdapter> + 'static,
    action: impl Widget<UIDataAdapter> + 'static,
) -> impl Widget<UIDataAdapter> {
    Flex::row()
        .with_flex_spacer(1.0)
        .with_child(cancel)
        .with_spacer(8.0)
        .with_child(action)
        .expand_width()
}

fn cancel_btn() -> impl Widget<UIDataAdapter> {
    centered_btn(
        "button.cancel",
        90.0,
        30.0,
        |t| t.btn_reset_bg,
        |t| t.btn_reset_text,
        Some(|t| t.btn_reset_border),
    )
}

fn general_tab() -> impl Widget<UIDataAdapter> {
    let input_mode_card = settings_card(
        Flex::column()
            .with_child(settings_row(
                "general.vietnamese_input",
                "general.enable_vietnamese",
                ToggleSwitch.lens(UIDataAdapter::is_enabled).on_click(
                    |_, data: &mut UIDataAdapter, _| {
                        data.toggle_vietnamese();
                    },
                ),
            ))
            .with_child(card_divider())
            .with_child(
                Flex::column()
                    .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                    .with_child(title_label("general.input_method"))
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
        "general.w_literal",
        "general.w_literal_desc",
        ToggleSwitch.lens(UIDataAdapter::is_w_literal_enabled),
    ));

    let system_card = settings_card(settings_row(
        "general.launch_at_login",
        "general.launch_at_login_desc",
        StyledCheckbox.lens(UIDataAdapter::launch_on_login),
    ));

    let language_card = option_group(
        title_subtitle_column("general.ui_language", "general.ui_language_desc"),
        U32SegmentedControl::new(vec![("Auto", 0), ("Tiếng Việt", 1), ("English", 2)])
            .lens(UIDataAdapter::ui_language),
    );

    let edit_shortcut_btn = Painter::new(|ctx, _: &UIDataAdapter, env| {
        let theme = theme_from_env(env);
        let size = ctx.size();
        let cx = size.width / 2.0;
        let cy = size.height / 2.0;
        let mut pencil = druid::kurbo::BezPath::new();
        pencil.move_to((cx - 1.5, cy + 6.0));
        pencil.line_to((cx - 6.0, cy + 1.5));
        pencil.line_to((cx + 1.5, cy - 6.0));
        pencil.line_to((cx + 6.0, cy - 1.5));
        pencil.close_path();
        ctx.fill(pencil, &theme.text_secondary);

        let mut nib = druid::kurbo::BezPath::new();
        nib.move_to((cx + 1.5, cy - 6.0));
        nib.line_to((cx + 6.0, cy - 1.5));
        nib.line_to((cx + 7.5, cy - 3.0));
        nib.line_to((cx + 3.0, cy - 7.5));
        nib.close_path();
        ctx.fill(nib, &theme.text_primary);

        let mut tip = druid::kurbo::BezPath::new();
        tip.move_to((cx - 1.5, cy + 6.0));
        tip.line_to((cx - 6.0, cy + 1.5));
        tip.line_to((cx - 8.0, cy + 8.0));
        tip.close_path();
        ctx.fill(tip, &theme.text_secondary);
    })
    .fix_size(24.0, 24.0)
    .on_click(|ctx, _: &mut UIDataAdapter, _| {
        ctx.submit_command(SHOW_EDIT_SHORTCUT_DIALOG.to(druid::Target::Global));
    });

    let shortcut_card = settings_card(settings_row(
        "general.toggle_shortcut",
        "general.toggle_shortcut_desc",
        Flex::row()
            .with_child(HotkeyBadgesWidget::new())
            .with_spacer(8.0)
            .with_child(edit_shortcut_btn),
    ));

    let footer = dialog_buttons(
        centered_btn(
            "general.reset_defaults",
            120.0,
            30.0,
            |t| t.btn_reset_bg,
            |t| t.btn_reset_text,
            Some(|t| t.btn_reset_border),
        )
        .on_click(|ctx, _data: &mut UIDataAdapter, _env| {
            ctx.submit_command(super::selectors::RESET_DEFAULTS);
        }),
        centered_btn(
            "general.done",
            70.0,
            30.0,
            |_| GREEN,
            |_| Color::WHITE,
            None,
        )
        .on_click(|ctx, _data: &mut UIDataAdapter, _env| {
            ctx.window().hide();
        }),
    );

    tab_body()
        .with_child(section_label("general.input_mode"))
        .with_child(input_mode_card)
        .with_spacer(8.0)
        .with_child(w_literal_card)
        .with_spacer(20.0)
        .with_child(section_label("general.system"))
        .with_child(system_card)
        .with_spacer(8.0)
        .with_child(language_card)
        .with_spacer(20.0)
        .with_child(section_label("general.shortcut"))
        .with_child(shortcut_card)
        .with_flex_spacer(1.0)
        .with_child(footer)
        .padding(TAB_PADDING)
}

fn apps_tab() -> impl Widget<UIDataAdapter> {
    let description = title_label("apps.description");

    let legend = Painter::new(|ctx, _: &UIDataAdapter, env| {
        let theme = theme_from_env(env);
        let mut x = 0.0;
        let bh = 22.0;
        let badge_y = (26.0 - bh) / 2.0;

        for (badge_text, badge_bg, badge_border, label_key) in [
            ("VI", BADGE_VI_BG, BADGE_VI_BORDER, "apps.vietnamese"),
            ("EN", BADGE_EN_BG, BADGE_EN_BORDER, "apps.english"),
        ] {
            let badge_layout = make_text_layout(ctx, badge_text, 11.0, &badge_border);
            let bw = badge_layout.size().width + 14.0;
            let rr = RoundedRect::new(x, badge_y, x + bw, badge_y + bh, 5.0);
            ctx.fill(rr, &badge_bg);
            ctx.stroke(rr, &badge_border, 1.0);
            ctx.draw_text(
                &badge_layout,
                (
                    x + (bw - badge_layout.size().width) / 2.0,
                    badge_y + (bh - badge_layout.size().height) / 2.0,
                ),
            );
            x += bw + 8.0;

            let label = make_text_layout(ctx, t(label_key), 13.0, &theme.text_primary);
            ctx.draw_text(&label, (x, (26.0 - label.size().height) / 2.0));
            x += label.size().width + 20.0;
        }
    })
    .fix_height(26.0)
    .expand_width();

    let add_btn = symbol_btn("+").on_click(|_, _, _| {
        defer_open_app_file_picker(Box::new(|name| {
            if let Some(name) = name {
                if let Some(sink) = UI_EVENT_SINK.get() {
                    let _ = sink.submit_command(SET_EN_APP_FROM_PICKER, name, Target::Auto);
                }
            }
        }));
    });

    let remove_btn =
        remove_btn(|d| d.selected_app_index >= 0).on_click(|ctx, data: &mut UIDataAdapter, _| {
            if data.selected_app_index >= 0 {
                ctx.submit_command(DELETE_SELECTED_APP.to(Target::Global));
            }
        });

    let card = list_card(
        AppsListWidget::new(),
        Flex::row()
            .with_child(add_btn)
            .with_child(remove_btn)
            .with_flex_spacer(1.0),
    );

    let per_app_toggle_card = settings_card(settings_row(
        "apps.per_app_toggle",
        "apps.per_app_toggle_desc",
        ToggleSwitch.lens(UIDataAdapter::is_auto_toggle_enabled),
    ));

    tab_body()
        .with_child(description)
        .with_spacer(12.0)
        .with_child(per_app_toggle_card)
        .with_spacer(16.0)
        .with_child(legend)
        .with_spacer(12.0)
        .with_flex_child(card.expand_height(), 1.0)
        .must_fill_main_axis(true)
        .expand()
        .padding(TAB_PADDING)
}

fn advanced_tab() -> impl Widget<UIDataAdapter> {
    let description = title_label("macro.description");

    let enable_row = settings_card(settings_row(
        "macro.text_expansion",
        "macro.enable",
        ToggleSwitch.lens(UIDataAdapter::is_macro_enabled),
    ));

    let autocap_row = settings_card(settings_row(
        "macro.auto_capitalize",
        "macro.auto_capitalize_desc",
        Flex::row()
            .with_child(
                InfoTooltip::new("ko → không\nKo → Không\nKO → KHÔNG")
                    .padding((0.0, 0.0, 8.0, 0.0)),
            )
            .with_child(StyledCheckbox.lens(UIDataAdapter::is_macro_autocap_enabled)),
    ));

    let add_btn = symbol_btn("+").on_click(|ctx, _data: &mut UIDataAdapter, _| {
        ctx.submit_command(SHOW_ADD_MACRO_DIALOG.to(Target::Global));
    });

    let remove_btn =
        remove_btn(|d| d.selected_macro_index >= 0).on_click(|ctx, data: &mut UIDataAdapter, _| {
            if data.selected_macro_index >= 0 {
                ctx.submit_command(DELETE_SELECTED_MACRO.to(Target::Global));
            }
        });

    let card = list_card(
        MacroListWidget::new(),
        Flex::row()
            .with_child(add_btn)
            .with_child(remove_btn)
            .with_flex_spacer(1.0)
            .with_child(toolbar_btn("macro.load", Some("left")).on_click(
                |ctx, _data: &mut UIDataAdapter, _| {
                    ctx.submit_command(LOAD_MACROS_FROM_FILE.to(Target::Global));
                },
            ))
            .with_child(toolbar_btn("macro.export", None).on_click(
                |ctx, _data: &mut UIDataAdapter, _| {
                    ctx.submit_command(EXPORT_MACROS_TO_FILE.to(Target::Global));
                },
            )),
    );

    tab_body()
        .with_child(description)
        .with_spacer(12.0)
        .with_child(enable_row)
        .with_spacer(8.0)
        .with_child(autocap_row)
        .with_spacer(16.0)
        .with_flex_child(card.expand_height(), 1.0)
        .must_fill_main_axis(true)
        .expand()
        .padding(TAB_PADDING)
}

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

pub fn main_ui_builder() -> impl Widget<UIDataAdapter> {
    let inner = Flex::column()
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
        .background(Painter::new(|ctx, _, env| {
            let theme = theme_from_env(env);
            let size = ctx.size();
            ctx.fill(size.to_rect(), &theme.win_bg);
        }))
        .controller(UIController);

    EnvScope::new(
        |env, data: &UIDataAdapter| {
            env.set(IS_DARK.clone(), data.is_dark);
        },
        inner,
    )
}

pub fn permission_request_ui_builder() -> impl Widget<()> {
    let image_data = ImageBuf::from_data(include_bytes!("../../assets/accessibility.png")).unwrap();

    let title_label = Label::new(t("perm.title"))
        .with_text_color(Color::rgb8(17, 17, 17))
        .with_font(druid::FontDescriptor::new(FontFamily::SYSTEM_UI).with_size(13.0))
        .with_line_break_mode(LineBreaking::WordWrap);

    let img_container = Container::new(Image::new(image_data).fill_mode(FillStrat::Cover))
        .rounded(8.0)
        .border(Color::rgba8(0, 0, 0, 30), 1.0);

    let subtitle_label = Label::new(t("perm.subtitle"))
        .with_text_color(Color::rgb8(102, 102, 102))
        .with_font(druid::FontDescriptor::new(FontFamily::SYSTEM_UI).with_size(12.0))
        .with_line_break_mode(LineBreaking::WordWrap);

    let exit_btn = Painter::new(|ctx, _: &(), _| {
        let size = ctx.size();
        let rr = RoundedRect::new(0.0, 0.0, size.width, size.height, 7.0);
        ctx.fill(rr, &GREEN);
        draw_centered_text(ctx, t("perm.exit"), 13.0, &Color::WHITE);
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
        .background(Color::rgb8(255, 255, 255))
}

pub fn macro_editor_ui_builder() -> impl Widget<UIDataAdapter> {
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_flex_child(
            v_scroll(
                List::new(macro_row_item)
                    .lens(UIDataAdapter::macro_table)
                    .expand_width(),
            )
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

pub const ADD_MACRO_DIALOG_WIDTH: f64 = 340.0;
pub const ADD_MACRO_DIALOG_HEIGHT: f64 = 208.0;

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
    let shorthand_label = subtitle_label("macro.shorthand");
    let replacement_label = subtitle_label("macro.replacement");

    let buttons = dialog_buttons(
        cancel_btn().on_click(|ctx, data: &mut UIDataAdapter, _| {
            data.new_macro_from = String::new();
            data.new_macro_to = String::new();
            ctx.window().close();
        }),
        action_btn("button.add", |d| {
            !d.new_macro_from.is_empty() && !d.new_macro_to.is_empty()
        })
        .on_click(|ctx, data: &mut UIDataAdapter, _| {
            if !data.new_macro_from.is_empty() && !data.new_macro_to.is_empty() {
                ctx.submit_command(ADD_MACRO.to(Target::Global));
                ctx.window().close();
            }
        }),
    );

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(shorthand_label)
        .with_spacer(4.0)
        .with_child(
            Container::new(styled_text_input("nope").lens(UIDataAdapter::new_macro_from))
                .border(Color::rgb8(204, 204, 204), 1.0)
                .rounded(8.0)
                .expand_width(),
        )
        .with_spacer(12.0)
        .with_child(replacement_label)
        .with_spacer(4.0)
        .with_child(
            Container::new(styled_text_input("dạ, thưa sếp").lens(UIDataAdapter::new_macro_to))
                .border(Color::rgb8(204, 204, 204), 1.0)
                .rounded(8.0)
                .expand_width(),
        )
        .with_flex_spacer(1.0)
        .with_child(buttons)
        .padding((24.0, 20.0, 24.0, 20.0))
        .background(Painter::new(|ctx, _, env| {
            let theme = theme_from_env(env);
            let size = ctx.size();
            ctx.fill(size.to_rect(), &theme.win_bg);
        }))
        .expand()
}

pub fn edit_shortcut_dialog_ui_builder() -> impl Widget<UIDataAdapter> {
    use super::selectors::SAVE_SHORTCUT;

    let title_label = subtitle_label("shortcut.new");
    let hint_label = text_label("shortcut.hint", 11.0, |t| t.text_secondary, 14.0);

    let buttons = dialog_buttons(
        cancel_btn().on_click(|ctx, _: &mut UIDataAdapter, _| {
            ctx.window().close();
        }),
        action_btn("button.save", |d| !d.pending_shortcut_display.is_empty()).on_click(
            |ctx, data: &mut UIDataAdapter, _| {
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
            },
        ),
    );

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(title_label)
        .with_spacer(4.0)
        .with_child(hint_label)
        .with_spacer(16.0)
        .with_child(
            Container::new(ShortcutCaptureWidget::new())
                .border(Color::rgb8(204, 204, 204), 1.0)
                .rounded(8.0)
                .fix_height(52.0)
                .expand_width(),
        )
        .with_flex_spacer(1.0)
        .with_child(buttons)
        .padding((24.0, 20.0, 24.0, 20.0))
        .background(Painter::new(|ctx, _, env| {
            let theme = theme_from_env(env);
            let size = ctx.size();
            ctx.fill(size.to_rect(), &theme.win_bg);
        }))
        .expand()
}
