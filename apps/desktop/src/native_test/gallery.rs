//! The `show_gallery` MCP command: a native check shows the design-system gallery, every token
//! drawn in the current mode, in place of the window's content.

use gpui_kit::{App, Entity};
use gpui_mcp::{
    ApplicationCommandDescriptor, ApplicationCommandRequest, ApplicationCommandResponse,
    ApplicationCommandResult, BridgeError, ErrorCode,
};
use skillbinder_ui::NativeView;

pub(super) const NAME: &str = "show_gallery";

const SCHEMA: &str = r#"{
  "type": "object",
  "additionalProperties": false,
  "required": ["show"],
  "properties": {
    "show": {
      "type": "boolean",
      "description": "true replaces the window's content with the gallery; false shows the app again. The gallery's Light and Dark buttons switch the mode."
    }
  }
}"#;

pub(super) fn descriptor() -> Result<ApplicationCommandDescriptor, BridgeError> {
    Ok(ApplicationCommandDescriptor {
        name: NAME.to_owned(),
        title: "Show design-system gallery".to_owned(),
        description: "Shows or hides the design-system gallery of tokens.".to_owned(),
        input_schema: SCHEMA.parse().map_err(|_| invalid("schema"))?,
        mutating: true,
    })
}

pub(super) fn execute(
    view: &Entity<NativeView>,
    request: ApplicationCommandRequest,
    cx: &mut App,
) -> Result<ApplicationCommandResponse, BridgeError> {
    let ApplicationCommandRequest::Execute { name, arguments } = request else {
        return Err(invalid("request"));
    };
    let show = arguments
        .get("show")
        .and_then(|value| value.as_bool())
        .ok_or_else(|| invalid("show"))?;
    view.update(cx, |view, cx| view.show_gallery(show, cx));
    Ok(ApplicationCommandResponse::Result(
        ApplicationCommandResult {
            name,
            revision: None,
            output: format!(r#"{{"show":{show}}}"#)
                .parse()
                .map_err(|_| invalid("output"))?,
        },
    ))
}

fn invalid(what: &str) -> BridgeError {
    BridgeError::new(
        ErrorCode::InvalidRequest,
        format!("Invalid show_gallery {what}."),
    )
}
