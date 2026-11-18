//! The logo an agent or a skill directory gets.

use super::super::library_copies::Reader;
use super::{agent_logo, reader_logo};
use skillbinder_design::Logo;

#[test]
fn agents_take_their_color_logo_then_their_monochrome_one_then_the_terminal_glyph() {
    assert_eq!(
        agent_logo("claude-code"),
        Logo::Color("agents/claude-code-color.svg")
    );
    assert_eq!(agent_logo("cursor"), Logo::Mono("agents/cursor.svg"));
    assert_eq!(
        agent_logo("dexto"),
        Logo::Glyph("icons/square-terminal.svg")
    );
    assert_eq!(
        reader_logo(&Reader::Universal(Vec::new())),
        Logo::Glyph("icons/layers.svg")
    );
}
