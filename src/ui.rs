use druid::{Lens, Data, Widget, widget::{Label, Button, Flex, Controller}, WidgetExt, Selector, EventCtx, Event, Env};
use crate::input::{INPUT_STATE, InputState};

pub const UPDATE_UI: Selector = Selector::new("gox-ui.update-ui");

#[derive(Clone, Data, Lens)]
pub struct GoxData {
    input_status: String
}

impl GoxData {
    pub fn new() -> Self {
        let mut ret = Self {
            input_status: String::new()
        };
        let input_state = INPUT_STATE.lock().unwrap();
        ret.update(&input_state);
        ret
    }

    pub fn update(&mut self, input_state: &InputState) {
        self.input_status = format!("Gõ tiếng Việt = {}", if input_state.enabled { "ON" } else { "OFF" });
    }

    pub fn toggle_input(&mut self) {
        let mut input_state = INPUT_STATE.lock().unwrap();
        input_state.toggle_vietnamese();
        self.update(&input_state);
    }
}

pub struct GoxUIController;

impl<W: Widget<GoxData>> Controller<GoxData, W> for GoxUIController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut GoxData,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) => {
                match cmd.get(UPDATE_UI) {
                    Some(_) => {
                        let input_state = INPUT_STATE.lock().unwrap();
                        data.update(&input_state);
                    },
                    None => {}
                }
            },
            _ => {}
        }
        child.event(ctx, event, data, env)
    }
}

pub fn main_ui_builder() -> impl Widget<GoxData> {
    let label = Label::new(|data: &String, _env: &_| data.clone())
        .lens(GoxData::input_status)
        .padding(5.0);
    let button = Button::new("Chuyển chế độ gõ")
        .on_click(|_, data: &mut GoxData, _| {
            data.toggle_input();
        })
        .padding(5.0);
    Flex::column()
        .with_child(label)
        .with_child(button)
        .padding(5.0)
        .controller(GoxUIController)
}
