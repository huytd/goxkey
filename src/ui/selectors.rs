use druid::Selector;

pub(super) const DELETE_MACRO: Selector<String> = Selector::new("gox-ui.delete-macro");
pub(super) const ADD_MACRO: Selector = Selector::new("gox-ui.add-macro");
pub(super) const DELETE_SELECTED_MACRO: Selector = Selector::new("gox-ui.delete-selected-macro");
pub(super) const SET_EN_APP_FROM_PICKER: Selector<String> =
    Selector::new("gox-ui.set-en-app-from-picker");
pub(super) const DELETE_SELECTED_APP: Selector = Selector::new("gox-ui.delete-selected-app");
pub(super) const TOGGLE_APP_MODE: Selector<String> = Selector::new("gox-ui.toggle-app-mode");
