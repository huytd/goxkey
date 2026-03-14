use druid::Selector;

pub(super) const DELETE_MACRO: Selector<String> = Selector::new("gox-ui.delete-macro");
pub(super) const ADD_MACRO: Selector = Selector::new("gox-ui.add-macro");
pub(super) const DELETE_VN_APP: Selector<String> = Selector::new("gox-ui.delete-vn-app");
pub(super) const DELETE_EN_APP: Selector<String> = Selector::new("gox-ui.delete-en-app");
pub(super) const ADD_VN_APP: Selector = Selector::new("gox-ui.add-vn-app");
pub(super) const ADD_EN_APP: Selector = Selector::new("gox-ui.add-en-app");
pub(super) const SET_VN_APP_FROM_PICKER: Selector<String> =
    Selector::new("gox-ui.set-vn-app-from-picker");
pub(super) const SET_EN_APP_FROM_PICKER: Selector<String> =
    Selector::new("gox-ui.set-en-app-from-picker");
pub(super) const DELETE_SELECTED_APP: Selector = Selector::new("gox-ui.delete-selected-app");
