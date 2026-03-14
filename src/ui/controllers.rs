use crate::{
    input::{rebuild_keyboard_layout_map, INPUT_STATE},
    platform::{update_launch_on_login, KeyModifier},
};
use druid::{Env, Event, EventCtx, UpdateCtx, Widget};
use log::error;

use super::{
    data::UIDataAdapter,
    format_letter_key, letter_key_to_char,
    selectors::{
        ADD_MACRO, DELETE_MACRO, DELETE_SELECTED_APP, SET_EN_APP_FROM_PICKER, TOGGLE_APP_MODE,
    },
    SHOW_UI, UPDATE_UI,
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

            if old_data.is_auto_toggle_enabled != data.is_auto_toggle_enabled {
                INPUT_STATE.toggle_auto_toggle();
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
