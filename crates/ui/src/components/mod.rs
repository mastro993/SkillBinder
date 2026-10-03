//! Shared native presentation primitives. Feature screens use the same semantic theme.
pub(crate) mod review;
use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme, button::Button, skeleton::Skeleton};
use skillbinder_app::models::{
    CandidateDuplicate, ImportOutcome, ValidationStatus, ValidationSummary,
};

pub fn column() -> Div {
    div().flex().flex_col().gap_3().min_w_0()
}
pub fn row() -> Div {
    div().flex().items_center().gap_3().flex_wrap()
}
pub fn panel(cx: &App) -> Div {
    column()
        .p_5()
        .rounded_lg()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
}
pub fn heading(title: impl Into<SharedString>, detail: impl Into<SharedString>, cx: &App) -> Div {
    column()
        .mb_5()
        .child(
            div()
                .text_3xl()
                .font_weight(FontWeight::SEMIBOLD)
                .child(title.into()),
        )
        .child(muted(detail, cx))
}
pub fn muted(text: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(text.into())
}
pub fn notice(text: impl Into<SharedString>, cx: &App) -> Div {
    panel(cx).bg(cx.theme().muted).text_sm().child(text.into())
}
pub fn button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
    let label = label.into();
    Button::new(id)
        .label(label.clone())
        .debug_selector(|| label.to_string())
}
pub fn loading(cx: &App) -> Div {
    column()
        .child(muted("Loading…", cx))
        .children((0..3).map(|_| Skeleton::new().h_20().w_full()))
}
pub fn validation(summary: &ValidationSummary, cx: &App) -> Div {
    let (label, color) = match summary.status {
        ValidationStatus::Valid => ("Valid", cx.theme().success),
        ValidationStatus::Warning => ("Warning", cx.theme().warning),
        ValidationStatus::Invalid => ("Invalid", cx.theme().danger),
        ValidationStatus::Blocked => ("Blocked", cx.theme().danger),
    };
    column()
        .gap_1()
        .child(div().text_sm().text_color(color).child(label))
        .children(
            summary
                .messages
                .iter()
                .map(|m| muted(m.message.clone(), cx)),
        )
}
pub fn duplicate(value: &CandidateDuplicate) -> String {
    match value {
        CandidateDuplicate::Unique => "New skill".into(),
        CandidateDuplicate::Identical { slug, .. } => format!("Identical to {slug}"),
        CandidateDuplicate::SlugInUse { slug, .. } => format!("Slug in use: {slug}"),
    }
}
pub fn outcome(value: &ImportOutcome) -> String {
    match value {
        ImportOutcome::NewSkill => "New skill".into(),
        ImportOutcome::AttachObservation { .. } => "Attach source to the existing skill".into(),
        ImportOutcome::Conflict { skill_ids } => format!(
            "Adds a second copy: {} skills already use this slug",
            skill_ids.len()
        ),
    }
}
