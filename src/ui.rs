use crate::input::{InputState, TypingMethod, INPUT_STATE};
use druid::{
    theme::{BACKGROUND_DARK, BORDER_DARK, PLACEHOLDER_COLOR},
    widget::{Button, Container, Controller, Flex, Label, RadioGroup, Switch},
    Data, Env, Event, EventCtx, Lens, Selector, Widget, WidgetExt,
};

pub const UPDATE_UI: Selector = Selector::new("gox-ui.update-ui");

#[derive(Clone, Data, Lens)]
pub struct GoxData {
    is_enabled: bool,
    typing_method: TypingMethod,
}

impl GoxData {
    pub fn new() -> Self {
        let mut ret = Self {
            is_enabled: true,
            typing_method: TypingMethod::Telex,
        };
        let input_state = INPUT_STATE.lock().unwrap();
        ret.update(&input_state);
        ret
    }

    pub fn update(&mut self, input_state: &InputState) {
        self.is_enabled = input_state.enabled;
        self.typing_method = input_state.method;
    }

    pub fn toggle_vietnamese(&mut self) {
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
            Event::Command(cmd) => match cmd.get(UPDATE_UI) {
                Some(_) => {
                    let input_state = INPUT_STATE.lock().unwrap();
                    data.update(&input_state);
                }
                None => {}
            },
            _ => {}
        }
        child.event(ctx, event, data, env)
    }
}

pub fn main_ui_builder() -> impl Widget<GoxData> {
    Flex::column()
        .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
        .main_axis_alignment(druid::widget::MainAxisAlignment::Start)
        .with_child(
            Container::new(
                Flex::column()
                    .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                    .main_axis_alignment(druid::widget::MainAxisAlignment::Start)
                    .with_child(
                        Flex::row()
                            .with_child(Label::new("Chế độ gõ tiếng Việt"))
                            .with_child(Switch::new().lens(GoxData::is_enabled).on_click(
                                |_, data, _| {
                                    data.toggle_vietnamese();
                                },
                            ))
                            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                            .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
                            .must_fill_main_axis(true)
                            .expand_width()
                            .padding(8.0),
                    )
                    .with_child(
                        Flex::row()
                            .with_child(Label::new("Kiểu gõ"))
                            .with_child(
                                RadioGroup::new(vec![
                                    ("Telex", TypingMethod::Telex),
                                    ("VNI", TypingMethod::VNI),
                                ])
                                .lens(GoxData::typing_method),
                            )
                            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                            .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
                            .must_fill_main_axis(true)
                            .expand_width()
                            .padding(8.0),
                    )
                    .with_child(
                        Flex::row()
                            .with_child(Label::new("Bật tắt gõ tiếng Việt"))
                            .with_child(
                                Label::new("⌃ ⌘ Space")
                                    .border(PLACEHOLDER_COLOR, 1.0)
                                    .rounded(4.0),
                            )
                            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                            .main_axis_alignment(druid::widget::MainAxisAlignment::SpaceBetween)
                            .must_fill_main_axis(true)
                            .expand_width()
                            .padding(8.0),
                    ),
            )
            .border(BORDER_DARK, 1.0)
            .rounded(4.0)
            .background(BACKGROUND_DARK),
        )
        .with_spacer(8.0)
        .with_child(
            Flex::row()
                .with_child(Button::new("Cài đặt mặc định").fix_height(28.0))
                .with_spacer(8.0)
                .with_child(Button::new("Đóng").fix_width(100.0).fix_height(28.0))
                .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
                .main_axis_alignment(druid::widget::MainAxisAlignment::End)
                .must_fill_main_axis(true)
                .expand_width(),
        )
        .padding(8.0)
        .controller(GoxUIController)
}
