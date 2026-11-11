//! The logo an agent or a skill directory gets.

use super::super::library_copies::Reader;
use super::Logo;

#[test]
fn agents_take_their_color_logo_then_their_monochrome_one_then_the_terminal_glyph() {
    assert_eq!(
        Logo::agent("claude-code"),
        Logo::Color("agents/claude-code-color.svg")
    );
    assert_eq!(Logo::agent("cursor"), Logo::Mono("agents/cursor.svg"));
    assert_eq!(
        Logo::agent("dexto"),
        Logo::Glyph("icons/square-terminal.svg")
    );
    assert_eq!(
        Logo::reader(&Reader::Universal(Vec::new())),
        Logo::Glyph("icons/layers.svg")
    );
}
