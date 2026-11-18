//! The page panel in points, on the shared scales: a raised [`crate::Surface::panel`] inset from
//! the window, holding every page under the header.

use crate::tokens::space;

/// Space between the panel and the header, and the window's right and bottom edges; its left
/// edge too while the sidebar is hidden.
pub const INSET: f32 = space::SM;
/// Space between the panel's edges and the page's content, on every side.
pub const GUTTER: f32 = space::XL;
/// Space between a page's blocks, such as Library's toolbar and its list.
pub const GAP: f32 = space::LG;
