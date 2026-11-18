//! Molecule: a notice, a `Sm` glyph and a line of text in a status `-text` tone (or muted) on no
//! fill, wrapping beside the glyph. A blocking problem is Kit's alert instead.

use crate::tokens::{space, text};
use crate::{IconSize, Spinner, Status, Text, TypeLevel, icon};
use gpui_kit::component::{
    ActiveTheme, Sizable,
    alert::{Alert, AlertVariant},
};
use gpui_kit::{
    App, ElementId, Hsla, IntoElement, ParentElement, Refineable, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div, px,
};

/// A notice's type size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NoticeSize {
    /// `caption`, under a row or beside a control.
    #[default]
    Sm,
    /// `body-sm`, a line of its own in a dialog or a page.
    Md,
}

impl NoticeSize {
    /// Both sizes.
    pub const ALL: [Self; 2] = [Self::Sm, Self::Md];

    fn level(self) -> TypeLevel {
        match self {
            Self::Sm => text::CAPTION,
            Self::Md => text::BODY_SM,
        }
    }
}

/// The glyph before the text.
enum Glyph {
    /// The status's own: `check-circle` for success, `alert-circle` for the others, none for a
    /// muted notice.
    Status,
    /// A retained glyph.
    Named(SharedString),
    /// A retained glyph turning, for work in flight.
    Turning(SharedString),
}

/// A notice: what happened, in its status's tone. Its id names its text, a label of its own.
#[derive(IntoElement)]
pub struct Notice {
    id: ElementId,
    style: StyleRefinement,
    status: Option<Status>,
    glyph: Glyph,
    title: Option<SharedString>,
    text: SharedString,
    size: NoticeSize,
    blocking: bool,
}

impl Notice {
    /// A muted notice `id` reading `text`, without a glyph until one is set.
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            status: None,
            glyph: Glyph::Status,
            title: None,
            text: text.into(),
            size: NoticeSize::default(),
            blocking: false,
        }
    }

    /// A blocking problem: Kit's alert, `title` above `text`, in `status`'s tone.
    pub fn blocking(
        id: impl Into<ElementId>,
        status: Status,
        title: impl Into<SharedString>,
        text: impl Into<SharedString>,
    ) -> Self {
        Self {
            title: Some(title.into()),
            blocking: true,
            ..Self::new(id, text).status(status)
        }
    }

    /// Sets the status, which sets the tone and the default glyph.
    pub fn status(self, status: Status) -> Self {
        Self {
            status: Some(status),
            ..self
        }
    }

    /// Shows the retained glyph `name`.
    pub fn glyph(self, name: impl Into<SharedString>) -> Self {
        Self {
            glyph: Glyph::Named(name.into()),
            ..self
        }
    }

    /// Turns the retained glyph `name`, for work in flight.
    pub fn turning(self, name: impl Into<SharedString>) -> Self {
        Self {
            glyph: Glyph::Turning(name.into()),
            ..self
        }
    }

    /// Sets the type size.
    pub fn size(self, size: NoticeSize) -> Self {
        Self { size, ..self }
    }

    /// The glyph's retained name, if it shows one.
    fn glyph_name(&self) -> Option<SharedString> {
        match (&self.glyph, self.status) {
            (Glyph::Named(name) | Glyph::Turning(name), _) => Some(name.clone()),
            (Glyph::Status, None) => None,
            (Glyph::Status, Some(Status::Success)) => Some("check-circle".into()),
            (Glyph::Status, Some(_)) => Some("alert-circle".into()),
        }
    }

    /// Kit's alert in the status's tone, its glyph the notice's or `alert-circle`.
    fn alert(self, tone: Hsla) -> Alert {
        let variant = match self.status {
            Some(Status::Info) | None => AlertVariant::Info,
            Some(Status::Warning) => AlertVariant::Warning,
            Some(Status::Danger) => AlertVariant::Error,
            Some(Status::Success) => AlertVariant::Success,
        };
        let glyph = self.glyph_name().unwrap_or_else(|| "alert-circle".into());
        let mut alert = Alert::new(self.id, self.text)
            .with_variant(variant)
            .small()
            .icon(icon(&glyph).with_size(IconSize::Sm))
            .text_color(tone);
        if let Some(title) = self.title {
            alert = alert.title(title);
        }
        alert.style().refine(&self.style);
        alert
    }
}

impl Styled for Notice {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Notice {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let tone = self
            .status
            .map_or(theme.muted_foreground, |status| status.text().of(theme));
        if self.blocking {
            return self.alert(tone).into_any_element();
        }
        let glyph = match (&self.glyph, self.glyph_name()) {
            (Glyph::Turning(_), Some(name)) => {
                Some(Spinner::new(IconSize::Sm).glyph(name).into_any_element())
            }
            (_, name) => name.map(|name| icon(&name).with_size(IconSize::Sm).into_any_element()),
        };
        let level = self.size.level();
        // Centers the glyph on the first line.
        let nudge = (level.size * level.line_height - IconSize::Sm.points()).max(0.) / 2.;
        let mut line = div();
        line.style().refine(&self.style);
        level
            .apply(line)
            .flex()
            .items_start()
            .gap(px(space::SM))
            .text_color(tone)
            .children(glyph.map(|glyph| div().flex_none().mt(px(nudge)).child(glyph)))
            .child(Text::labeled(self.id, self.text).min_w_0().flex_1())
            .into_any_element()
    }
}

#[cfg(test)]
mod tests;
