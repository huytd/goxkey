use crate::{
    input::TypingMethod,
    platform::{defer_open_app_file_picker, SYMBOL_ALT, SYMBOL_CTRL, SYMBOL_SHIFT, SYMBOL_SUPER},
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
    colors::{
        BADGE_EN_BG, BADGE_EN_BORDER, BADGE_VI_BG, BADGE_VI_BORDER, BTN_RESET_BG, BTN_RESET_BORDER,
        CARD_BG, CARD_BORDER, DIVIDER, GREEN, TEXT_PRIMARY, TEXT_SECONDARY, TEXT_SECTION, WIN_BG,
    },
    controllers::{LetterKeyController, UIController},
    data::{AppEntry, MacroEntry, UIDataAdapter},
    selectors::{ADD_MACRO, DELETE_MACRO, DELETE_SELECTED_APP, SET_EN_APP_FROM_PICKER},
    widgets::{
        AppsListWidget, HotkeyBadgesWidget, SegmentedControl, StyledCheckbox, TabBar, ToggleSwitch,
    },
    WINDOW_HEIGHT, WINDOW_WIDTH,
};

// ── Layout helpers ─────────────────────────────────────────────────────────────

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
        .with_flex_child(
            Flex::column()
                .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                .with_child(
                    Painter::new(move |ctx, _data: &UIDataAdapter, _env| {
                        let layout = ctx
                            .text()
                            .new_text_layout(title)
                            .font(FontFamily::SYSTEM_UI, 13.0)
                            .text_color(TEXT_PRIMARY)
                            .build()
                            .unwrap();
                        ctx.draw_text(&layout, (0.0, 0.0));
                    })
                    .fix_height(18.0)
                    .expand_width(),
                )
                .with_child(
                    Painter::new(move |ctx, _data: &UIDataAdapter, _env| {
                        let layout = ctx
                            .text()
                            .new_text_layout(subtitle)
                            .font(FontFamily::SYSTEM_UI, 12.0)
                            .text_color(TEXT_SECONDARY)
                            .build()
                            .unwrap();
                        ctx.draw_text(&layout, (0.0, 0.0));
                    })
                    .fix_height(16.0)
                    .expand_width(),
                ),
            1.0,
        )
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
                        Flex::column()
                            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                            .with_child(
                                Painter::new(|ctx, _data: &UIDataAdapter, _env| {
                                    let layout = ctx
                                        .text()
                                        .new_text_layout("Vietnamese input")
                                        .font(FontFamily::SYSTEM_UI, 13.0)
                                        .text_color(TEXT_PRIMARY)
                                        .build()
                                        .unwrap();
                                    ctx.draw_text(&layout, (0.0, 0.0));
                                })
                                .fix_height(18.0)
                                .expand_width(),
                            )
                            .with_child(
                                Painter::new(|ctx, _data: &UIDataAdapter, _env| {
                                    let layout = ctx
                                        .text()
                                        .new_text_layout("Enable Vietnamese typing mode")
                                        .font(FontFamily::SYSTEM_UI, 12.0)
                                        .text_color(TEXT_SECONDARY)
                                        .build()
                                        .unwrap();
                                    ctx.draw_text(&layout, (0.0, 0.0));
                                })
                                .fix_height(16.0)
                                .expand_width(),
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
                    .with_child(
                        Painter::new(|ctx, _data: &UIDataAdapter, _env| {
                            let layout = ctx
                                .text()
                                .new_text_layout("Input method")
                                .font(FontFamily::SYSTEM_UI, 13.0)
                                .text_color(TEXT_PRIMARY)
                                .build()
                                .unwrap();
                            ctx.draw_text(&layout, (0.0, 0.0));
                        })
                        .fix_height(18.0)
                        .expand_width(),
                    )
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

    let system_card = settings_card(
        Flex::column()
            .with_child(settings_row(
                "Launch at login",
                "Start gõkey when you log in",
                StyledCheckbox.lens(UIDataAdapter::launch_on_login),
            ))
            .with_child(card_divider())
            .with_child(settings_row(
                "Per-app toggle",
                "Enable/disable per application",
                StyledCheckbox.lens(UIDataAdapter::is_auto_toggle_enabled),
            )),
    );

    let shortcut_card = settings_card(
        Flex::row()
            .with_flex_child(
                Flex::column()
                    .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                    .with_child(
                        Painter::new(|ctx, _data: &UIDataAdapter, _env| {
                            let layout = ctx
                                .text()
                                .new_text_layout("Toggle Vietnamese input")
                                .font(FontFamily::SYSTEM_UI, 13.0)
                                .text_color(TEXT_PRIMARY)
                                .build()
                                .unwrap();
                            ctx.draw_text(&layout, (0.0, 0.0));
                        })
                        .fix_height(18.0)
                        .expand_width(),
                    )
                    .with_child(
                        Painter::new(|ctx, _data: &UIDataAdapter, _env| {
                            let layout = ctx
                                .text()
                                .new_text_layout("Keyboard shortcut to toggle on/off")
                                .font(FontFamily::SYSTEM_UI, 12.0)
                                .text_color(TEXT_SECONDARY)
                                .build()
                                .unwrap();
                            ctx.draw_text(&layout, (0.0, 0.0));
                        })
                        .fix_height(16.0)
                        .expand_width(),
                    ),
                1.0,
            )
            .with_child(HotkeyBadgesWidget::new())
            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Center)
            .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
            .must_fill_main_axis(true)
            .expand_width()
            .padding((14.0, 10.0)),
    );

    let footer = Flex::row()
        .with_flex_spacer(1.0)
        .with_child(
            Painter::new(|ctx, _data: &UIDataAdapter, _env| {
                let size = ctx.size();
                let rr = RoundedRect::new(0.0, 0.0, size.width, size.height, 7.0);
                ctx.fill(rr, &BTN_RESET_BG);
                ctx.stroke(rr, &BTN_RESET_BORDER, 0.5);
                let layout = ctx
                    .text()
                    .new_text_layout("Reset defaults")
                    .font(FontFamily::SYSTEM_UI, 13.0)
                    .text_color(Color::rgb8(51, 51, 51))
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
            .fix_size(120.0, 30.0),
        )
        .with_spacer(8.0)
        .with_child(
            Painter::new(|ctx, _data: &UIDataAdapter, _env| {
                let size = ctx.size();
                let rr = RoundedRect::new(0.0, 0.0, size.width, size.height, 7.0);
                ctx.fill(rr, &GREEN);
                let layout = ctx
                    .text()
                    .new_text_layout("Done")
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
            .on_click(|ctx, _data: &mut UIDataAdapter, _env| {
                ctx.window().hide();
            }),
        )
        .expand_width();

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(section_label("Input mode"))
        .with_child(input_mode_card)
        .with_spacer(20.0)
        .with_child(section_label("System"))
        .with_child(system_card)
        .with_spacer(20.0)
        .with_child(section_label("Shortcut"))
        .with_child(shortcut_card)
        .with_flex_spacer(1.0)
        .with_child(footer)
        .padding((24.0, 20.0, 24.0, 24.0))
}

fn apps_tab() -> impl Widget<UIDataAdapter> {
    let description = Painter::new(|ctx, _: &UIDataAdapter, _| {
        let layout = ctx
            .text()
            .new_text_layout("Set input language per application.")
            .font(FontFamily::SYSTEM_UI, 13.0)
            .text_color(TEXT_PRIMARY)
            .build()
            .unwrap();
        ctx.draw_text(&layout, (0.0, 0.0));
    })
    .fix_height(18.0)
    .expand_width();

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
            .new_text_layout("Vietnamese")
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
            .new_text_layout("English")
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

    let add_btn = Painter::new(|ctx, _: &UIDataAdapter, _| {
        let size = ctx.size();
        let layout = ctx
            .text()
            .new_text_layout("+")
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
    .on_click(|_, _, _| {
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
            .with_child(
                Painter::new(|ctx, _: &UIDataAdapter, _| {
                    let w = ctx.size().width;
                    ctx.fill(Rect::new(0.0, 0.0, w, 0.5), &DIVIDER);
                })
                .fix_height(0.5)
                .expand_width(),
            )
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

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(description)
        .with_spacer(12.0)
        .with_child(legend)
        .with_spacer(12.0)
        .with_flex_child(card.expand_height(), 1.0)
        .must_fill_main_axis(true)
        .expand()
        .padding((24.0, 20.0, 24.0, 24.0))
}

fn shortcuts_tab() -> impl Widget<UIDataAdapter> {
    let card = settings_card(
        Flex::column()
            .with_child(
                Flex::row()
                    .with_flex_child(
                        Flex::column()
                            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                            .with_child(
                                Painter::new(|ctx, _: &UIDataAdapter, _| {
                                    let layout = ctx
                                        .text()
                                        .new_text_layout("Toggle Vietnamese input")
                                        .font(FontFamily::SYSTEM_UI, 13.0)
                                        .text_color(TEXT_PRIMARY)
                                        .build()
                                        .unwrap();
                                    ctx.draw_text(&layout, (0.0, 0.0));
                                })
                                .fix_height(18.0)
                                .expand_width(),
                            )
                            .with_child(
                                Painter::new(|ctx, data: &UIDataAdapter, _| {
                                    let layout = ctx
                                        .text()
                                        .new_text_layout(data.hotkey_display.clone())
                                        .font(FontFamily::SYSTEM_UI, 12.0)
                                        .text_color(TEXT_SECONDARY)
                                        .build()
                                        .unwrap();
                                    ctx.draw_text(&layout, (0.0, 0.0));
                                })
                                .fix_height(16.0)
                                .expand_width(),
                            ),
                        1.0,
                    )
                    .cross_axis_alignment(druid::widget::CrossAxisAlignment::Center)
                    .expand_width()
                    .padding((14.0, 10.0)),
            )
            .with_child(card_divider())
            .with_child(
                Flex::column()
                    .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                    .with_child(
                        Painter::new(|ctx, _: &UIDataAdapter, _| {
                            let layout = ctx
                                .text()
                                .new_text_layout("Modifiers")
                                .font(FontFamily::SYSTEM_UI, 12.0)
                                .text_color(TEXT_SECONDARY)
                                .build()
                                .unwrap();
                            ctx.draw_text(&layout, (0.0, 0.0));
                        })
                        .fix_height(16.0)
                        .expand_width(),
                    )
                    .with_spacer(8.0)
                    .with_child(modifier_row())
                    .with_spacer(12.0)
                    .with_child(
                        Painter::new(|ctx, _: &UIDataAdapter, _| {
                            let layout = ctx
                                .text()
                                .new_text_layout("Key")
                                .font(FontFamily::SYSTEM_UI, 12.0)
                                .text_color(TEXT_SECONDARY)
                                .build()
                                .unwrap();
                            ctx.draw_text(&layout, (0.0, 0.0));
                        })
                        .fix_height(16.0)
                        .expand_width(),
                    )
                    .with_spacer(6.0)
                    .with_child(
                        TextBox::new()
                            .lens(UIDataAdapter::letter_key)
                            .controller(LetterKeyController)
                            .fix_width(80.0),
                    )
                    .expand_width()
                    .padding((14.0, 10.0)),
            ),
    );

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(section_label("Keyboard shortcut"))
        .with_child(card)
        .with_flex_spacer(1.0)
        .padding((24.0, 20.0, 24.0, 24.0))
}

fn modifier_row() -> impl Widget<UIDataAdapter> {
    fn modifier_checkbox(
        lens: impl druid::Lens<UIDataAdapter, bool> + 'static,
        symbol: &'static str,
    ) -> impl Widget<UIDataAdapter> {
        Flex::row()
            .with_child(StyledCheckbox.lens(lens).padding((0.0, 0.0, 6.0, 0.0)))
            .with_child(
                Painter::new(move |ctx, _: &UIDataAdapter, _| {
                    let layout = ctx
                        .text()
                        .new_text_layout(symbol)
                        .font(FontFamily::SYSTEM_UI, 13.0)
                        .text_color(TEXT_PRIMARY)
                        .build()
                        .unwrap();
                    ctx.draw_text(&layout, (0.0, 0.0));
                })
                .fix_height(20.0)
                .fix_width(24.0),
            )
            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Center)
    }

    Flex::row()
        .with_child(modifier_checkbox(UIDataAdapter::super_key, SYMBOL_SUPER))
        .with_spacer(12.0)
        .with_child(modifier_checkbox(UIDataAdapter::ctrl_key, SYMBOL_CTRL))
        .with_spacer(12.0)
        .with_child(modifier_checkbox(UIDataAdapter::alt_key, SYMBOL_ALT))
        .with_spacer(12.0)
        .with_child(modifier_checkbox(UIDataAdapter::shift_key, SYMBOL_SHIFT))
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Center)
}

fn advanced_tab() -> impl Widget<UIDataAdapter> {
    let macro_card = settings_card(
        Flex::column()
            .with_child(settings_row(
                "Text expansion",
                "Expand shorthand into full text",
                StyledCheckbox.lens(UIDataAdapter::is_macro_enabled),
            ))
            .with_child(card_divider())
            .with_child(
                Flex::column()
                    .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                    .with_child(
                        Painter::new(|ctx, _: &UIDataAdapter, _| {
                            let layout = ctx
                                .text()
                                .new_text_layout("Macro table")
                                .font(FontFamily::SYSTEM_UI, 12.0)
                                .text_color(TEXT_SECONDARY)
                                .build()
                                .unwrap();
                            ctx.draw_text(&layout, (0.0, 0.0));
                        })
                        .fix_height(16.0)
                        .expand_width(),
                    )
                    .with_spacer(8.0)
                    .with_flex_child(
                        {
                            let mut scroll = Scroll::new(
                                List::new(macro_row_item)
                                    .lens(UIDataAdapter::macro_table)
                                    .expand_width(),
                            );
                            scroll.set_enabled_scrollbars(
                                druid::scroll_component::ScrollbarsEnabled::Vertical,
                            );
                            scroll.set_horizontal_scroll_enabled(false);
                            scroll
                        }
                        .expand(),
                        1.0,
                    )
                    .with_spacer(8.0)
                    .with_child(
                        Flex::row()
                            .with_flex_child(
                                TextBox::new()
                                    .with_placeholder("Shorthand")
                                    .expand_width()
                                    .lens(UIDataAdapter::new_macro_from),
                                2.0,
                            )
                            .with_spacer(6.0)
                            .with_flex_child(
                                TextBox::new()
                                    .with_placeholder("Replacement")
                                    .expand_width()
                                    .lens(UIDataAdapter::new_macro_to),
                                2.0,
                            )
                            .with_spacer(6.0)
                            .with_child(Button::new("Add").on_click(|ctx, _, _| {
                                ctx.submit_command(ADD_MACRO.to(Target::Global))
                            }))
                            .expand_width(),
                    )
                    .expand_width()
                    .padding((14.0, 10.0)),
            ),
    );

    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .with_child(section_label("Text expansion"))
        .with_flex_child(macro_card.expand_height(), 1.0)
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

fn app_row_item(delete_selector: druid::Selector<String>) -> impl Widget<AppEntry> {
    Flex::row()
        .with_flex_child(
            Label::dynamic(|e: &AppEntry, _| {
                std::path::Path::new(&e.name)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&e.name)
                    .to_string()
            })
            .with_line_break_mode(LineBreaking::Clip)
            .align_left()
            .padding((4.0, 2.0)),
            1.0,
        )
        .with_child(Button::new("×").fix_width(28.0).on_click(
            move |ctx, data: &mut AppEntry, _| {
                ctx.submit_command(delete_selector.with(data.name.clone()).to(Target::Global))
            },
        ))
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Center)
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
                    2 => Box::new(shortcuts_tab()),
                    3 => Box::new(advanced_tab()),
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
    let image_data = ImageBuf::from_data(include_bytes!("../../assets/accessibility.png")).unwrap();
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .main_axis_alignment(druid::widget::MainAxisAlignment::Start)
        .with_child(
            Label::new("Chờ đã! Bạn cần phải cấp quyền Accessibility cho ứng dụng GõKey trước khi sử dụng.")
                .with_line_break_mode(LineBreaking::WordWrap)
                .padding(6.0),
        )
        .with_child(
            Container::new(Image::new(image_data).fill_mode(FillStrat::Cover))
                .rounded(4.0)
                .padding(6.0),
        )
        .with_child(
            Label::new("Bạn vui lòng thoát khỏi ứng dụng và mở lại sau khi đã cấp quyền.")
                .with_line_break_mode(LineBreaking::WordWrap)
                .padding(6.0),
        )
        .with_child(
            Flex::row()
                .cross_axis_alignment(druid::widget::CrossAxisAlignment::End)
                .main_axis_alignment(druid::widget::MainAxisAlignment::End)
                .with_child(
                    Button::new("Thoát")
                        .fix_width(100.0)
                        .fix_height(28.0)
                        .on_click(|_, _, _| Application::global().quit())
                        .padding(6.0),
                )
                .must_fill_main_axis(true),
        )
        .must_fill_main_axis(true)
        .padding(6.0)
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
                        .with_placeholder("Shorthand")
                        .expand_width()
                        .lens(UIDataAdapter::new_macro_from),
                    2.0,
                )
                .with_flex_child(
                    TextBox::new()
                        .with_placeholder("Replacement")
                        .expand_width()
                        .lens(UIDataAdapter::new_macro_to),
                    2.0,
                )
                .with_child(
                    Button::new("Add")
                        .on_click(|ctx, _, _| ctx.submit_command(ADD_MACRO.to(Target::Global))),
                )
                .expand_width(),
        )
        .padding(8.0)
}

pub fn app_settings_ui_builder() -> impl Widget<UIDataAdapter> {
    apps_tab()
}

pub fn center_window_position() -> (f64, f64) {
    let screen_rect = Screen::get_display_rect();
    let x = (screen_rect.width() - WINDOW_WIDTH) / 2.0;
    let y = (screen_rect.height() - WINDOW_HEIGHT) / 2.0;
    (x, y)
}
