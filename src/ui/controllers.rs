use crate::{
    input::{rebuild_keyboard_layout_map, INPUT_STATE},
    platform::{defer_open_text_file_picker, defer_save_text_file_picker, update_launch_on_login, KeyModifier},
};
use druid::{Env, Event, EventCtx, Screen, UpdateCtx, Widget, WindowDesc, WindowLevel};
use log::error;

use super::{
    add_macro_dialog_ui_builder, edit_shortcut_dialog_ui_builder,
    data::UIDataAdapter,
    format_letter_key, letter_key_to_char,
    selectors::{
        ADD_MACRO, DELETE_MACRO, DELETE_SELECTED_APP, DELETE_SELECTED_MACRO, EXPORT_MACROS_TO_FILE,
        LOAD_MACROS_FROM_FILE, RESET_DEFAULTS, SAVE_SHORTCUT, SET_EN_APP_FROM_PICKER,
        SHOW_ADD_MACRO_DIALOG, SHOW_EDIT_SHORTCUT_DIALOG, TOGGLE_APP_MODE,
    },
    SHOW_UI, UPDATE_UI, ADD_MACRO_DIALOG_HEIGHT, ADD_MACRO_DIALOG_WIDTH,
    EDIT_SHORTCUT_DIALOG_HEIGHT, EDIT_SHORTCUT_DIALOG_WIDTH,
};

pub struct UIController;

impl<W: Widget<UIDataAdapter>> druid::widget::Controller<UIDataAdapter, W> for UIController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut UIDataAdapter,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) => {
                if cmd.get(UPDATE_UI).is_some() {
                    data.update();
                    rebuild_keyboard_layout_map();
                }
                if cmd.get(SHOW_UI).is_some() {
                    ctx.set_handled();
                    ctx.window().bring_to_front_and_focus();
                }
                if let Some(source) = cmd.get(DELETE_MACRO) {
                    unsafe { INPUT_STATE.delete_macro(source) };
                    data.update();
                }
                if cmd.get(ADD_MACRO).is_some()
                    && !data.new_macro_from.is_empty()
                    && !data.new_macro_to.is_empty()
                {
                    unsafe {
                        INPUT_STATE
                            .add_macro(data.new_macro_from.clone(), data.new_macro_to.clone())
                    };
                    data.new_macro_from = String::new();
                    data.new_macro_to = String::new();
                    data.update();
                }
                if cmd.get(SHOW_ADD_MACRO_DIALOG).is_some() {
                    data.new_macro_from = String::new();
                    data.new_macro_to = String::new();
                    let screen = Screen::get_display_rect();
                    let x = (screen.width() - ADD_MACRO_DIALOG_WIDTH) / 2.0;
                    let y = (screen.height() - ADD_MACRO_DIALOG_HEIGHT) / 2.0;
                    let dialog = WindowDesc::new(add_macro_dialog_ui_builder())
                        .title("Add Text Expansion")
                        .window_size((ADD_MACRO_DIALOG_WIDTH, ADD_MACRO_DIALOG_HEIGHT))
                        .resizable(false)
                        .set_position((x, y))
                        .set_level(WindowLevel::Modal(ctx.window().clone()));
                    ctx.new_window(dialog);
                    ctx.set_handled();
                }
                if cmd.get(SHOW_EDIT_SHORTCUT_DIALOG).is_some() {
                    data.pending_shortcut_display = String::new();
                    data.pending_shortcut_super = false;
                    data.pending_shortcut_ctrl = false;
                    data.pending_shortcut_alt = false;
                    data.pending_shortcut_shift = false;
                    data.pending_shortcut_letter = String::new();
                    let screen = Screen::get_display_rect();
                    let x = (screen.width() - EDIT_SHORTCUT_DIALOG_WIDTH) / 2.0;
                    let y = (screen.height() - EDIT_SHORTCUT_DIALOG_HEIGHT) / 2.0;
                    let dialog = WindowDesc::new(edit_shortcut_dialog_ui_builder())
                        .title("Edit Shortcut")
                        .window_size((EDIT_SHORTCUT_DIALOG_WIDTH, EDIT_SHORTCUT_DIALOG_HEIGHT))
                        .resizable(false)
                        .set_position((x, y))
                        .set_level(WindowLevel::Modal(ctx.window().clone()));
                    ctx.new_window(dialog);
                    ctx.set_handled();
                }
                if let Some((is_super, is_ctrl, is_alt, is_shift, letter)) =
                    cmd.get(SAVE_SHORTCUT)
                {
                    let mut new_mod = KeyModifier::new();
                    new_mod.apply(*is_super, *is_ctrl, *is_alt, *is_shift, false);
                    let key_code = letter_key_to_char(letter);
                    unsafe {
                        INPUT_STATE.set_hotkey(&format!(
                            "{}{}",
                            new_mod,
                            match key_code {
                                Some(' ') => String::from("space"),
                                Some(c) => c.to_string(),
                                _ => String::new(),
                            }
                        ));
                    }
                    data.update();
                    ctx.set_handled();
                }
                if let Some(name) = cmd.get(SET_EN_APP_FROM_PICKER) {
                    data.new_en_app = name.clone();
                    // In the new Apps tab design, adding via picker immediately commits
                    unsafe { INPUT_STATE.add_english_app(&data.new_en_app.clone()) };
                    data.new_en_app = String::new();
                    data.update();
                }
                if let Some(app_name) = cmd.get(TOGGLE_APP_MODE) {
                    let is_vn = data.vn_apps.iter().any(|e| &e.name == app_name);
                    if is_vn {
                        unsafe {
                            INPUT_STATE.remove_vietnamese_app(app_name);
                            INPUT_STATE.add_english_app(app_name);
                        }
                    } else {
                        unsafe {
                            INPUT_STATE.remove_english_app(app_name);
                            INPUT_STATE.add_vietnamese_app(app_name);
                        }
                    }
                    data.update();
                }
                if cmd.get(DELETE_SELECTED_MACRO).is_some() {
                    let idx = data.selected_macro_index;
                    if idx >= 0 {
                        if let Some(entry) = data.macro_table.get(idx as usize) {
                            let source = entry.from.clone();
                            unsafe { INPUT_STATE.delete_macro(&source) };
                        }
                        data.selected_macro_index = -1;
                        data.update();
                    }
                }
                if cmd.get(LOAD_MACROS_FROM_FILE).is_some() {
                    ctx.set_handled();
                    let event_sink = unsafe { crate::UI_EVENT_SINK.get().cloned() };
                    defer_open_text_file_picker(Box::new(move |path| {
                        if let Some(path) = path {
                            unsafe {
                                let _ = INPUT_STATE.import_macros_from_file(&path);
                            }
                            if let Some(sink) = event_sink {
                                let _ = sink.submit_command(crate::ui::UPDATE_UI, (), druid::Target::Global);
                            }
                        }
                    }));
                }
                if cmd.get(EXPORT_MACROS_TO_FILE).is_some() {
                    ctx.set_handled();
                    let event_sink = unsafe { crate::UI_EVENT_SINK.get().cloned() };
                    defer_save_text_file_picker(Box::new(move |path| {
                        if let Some(path) = path {
                            unsafe {
                                let _ = INPUT_STATE.export_macros_to_file(&path);
                            }
                            if let Some(sink) = event_sink {
                                let _ = sink.submit_command(crate::ui::UPDATE_UI, (), druid::Target::Global);
                            }
                        }
                    }));
                }
                if cmd.get(DELETE_SELECTED_APP).is_some() {
                    let idx = data.selected_app_index;
                    if idx >= 0 {
                        let vn_len = data.vn_apps.len() as i32;
                        if idx < vn_len {
                            if let Some(entry) = data.vn_apps.get(idx as usize) {
                                let name = entry.name.clone();
                                unsafe { INPUT_STATE.remove_vietnamese_app(&name) };
                            }
                        } else {
                            let en_idx = (idx - vn_len) as usize;
                            if let Some(entry) = data.en_apps.get(en_idx) {
                                let name = entry.name.clone();
                                unsafe { INPUT_STATE.remove_english_app(&name) };
                            }
                        }
                        data.selected_app_index = -1;
                        data.update();
                    }
                }
                if cmd.get(RESET_DEFAULTS).is_some() {
                    unsafe {
                        if !INPUT_STATE.is_enabled() {
                            INPUT_STATE.toggle_vietnamese();
                        }
                        INPUT_STATE.set_method(crate::input::TypingMethod::Telex);
                        INPUT_STATE.set_hotkey("ctrl+space");
                    }
                    if let Err(err) = update_launch_on_login(true) {
                        error!("{}", err);
                    }
                    data.update();
                    ctx.set_handled();
                }
            }
            Event::WindowCloseRequested => {
                ctx.set_handled();
                ctx.window().hide();
            }
            _ => {}
        }
        child.event(ctx, event, data, env)
    }

    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old_data: &UIDataAdapter,
        data: &UIDataAdapter,
        env: &Env,
    ) {
        unsafe {
            if old_data.typing_method != data.typing_method {
                INPUT_STATE.set_method(data.typing_method);
            }

            if old_data.launch_on_login != data.launch_on_login {
                if let Err(err) = update_launch_on_login(data.launch_on_login) {
                    error!("{}", err);
                }
            }

            // Update hotkey
            {
                let mut new_mod = KeyModifier::new();
                new_mod.apply(
                    data.super_key,
                    data.ctrl_key,
                    data.alt_key,
                    data.shift_key,
                    data.capslock_key,
                );
                let key_code = letter_key_to_char(&data.letter_key);
                if !INPUT_STATE.get_hotkey().is_match(new_mod, key_code) {
                    INPUT_STATE.set_hotkey(&format!(
                        "{}{}",
                        new_mod,
                        match key_code {
                            Some(' ') => String::from("space"),
                            Some(c) => c.to_string(),
                            _ => String::new(),
                        }
                    ));
                }
            }

            if old_data.is_macro_enabled != data.is_macro_enabled {
                INPUT_STATE.toggle_macro_enabled();
            }

            if old_data.is_macro_autocap_enabled != data.is_macro_autocap_enabled {
                INPUT_STATE.toggle_macro_autocap();
            }

            if old_data.is_auto_toggle_enabled != data.is_auto_toggle_enabled {
                INPUT_STATE.toggle_auto_toggle();
            }

            if old_data.is_w_literal_enabled != data.is_w_literal_enabled {
                INPUT_STATE.toggle_w_literal();
            }

            if old_data.ui_language != data.ui_language {
                let lang_str = match data.ui_language {
                    1 => "vi",
                    2 => "en",
                    _ => "auto",
                };
                crate::config::CONFIG_MANAGER
                    .lock()
                    .unwrap()
                    .set_ui_language(lang_str);
                super::locale::init_lang(lang_str);
                ctx.request_paint();
                // Trigger data.update() so system tray menu text refreshes
                ctx.submit_command(super::UPDATE_UI);
            }
        }
        child.update(ctx, old_data, data, env);
    }
}

pub(super) struct LetterKeyController;

impl<W: Widget<UIDataAdapter>> druid::widget::Controller<UIDataAdapter, W> for LetterKeyController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut UIDataAdapter,
        env: &Env,
    ) {
        if let &Event::MouseDown(_) = event {
            ctx.submit_command(druid::commands::SELECT_ALL);
        }
        if let &Event::KeyUp(_) = event {
            match data.letter_key.as_str() {
                "Space" => {}
                s => {
                    data.letter_key = format_letter_key(letter_key_to_char(s));
                }
            }
        }
        child.event(ctx, event, data, env)
    }
}
