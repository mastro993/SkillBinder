//! The page column in points, on the shared scales: the flat column beside the sidebar that holds
//! the page header and every page under it.

use crate::tokens::space;

/// Space between the page column's edges and the page's content, on every side; the page header
/// uses it too.
pub const GUTTER: f32 = space::XL;
/// Space between a page's blocks, such as Library's toolbar and its list.
pub const GAP: f32 = space::LG;
