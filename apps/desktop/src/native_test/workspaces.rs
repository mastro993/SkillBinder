//! The `add_workspace` MCP command: a native check adds a folder's projects as workspaces without
//! the system folder panel, which it cannot drive.

use gpui_kit::{App, Entity};
use gpui_mcp::{
    ApplicationCommandDescriptor, ApplicationCommandRequest, ApplicationCommandResponse,
    ApplicationCommandResult, BridgeError, ErrorCode,
};
use skillbinder_ui::NativeView;
use std::path::PathBuf;

pub(super) const NAME: &str = "add_workspace";

const SCHEMA: &str = r#"{
  "type": "object",
  "additionalProperties": false,
  "required": ["path"],
  "properties": {
    "path": {
      "type": "string",
      "description": "Absolute path of the folder to add, as if picked in the folder panel. Its direct children with an agent skill directory become workspaces, or the folder alone when none has one. A refusal shows in the window as an error."
    }
  }
}"#;

/// `add_workspace`'s listing.
pub(super) fn descriptor() -> Result<ApplicationCommandDescriptor, BridgeError> {
    Ok(ApplicationCommandDescriptor {
        name: NAME.to_owned(),
        title: "Add workspace folder".to_owned(),
        description: "Adds a folder's projects as workspaces without the system folder panel."
            .to_owned(),
        input_schema: SCHEMA.parse().map_err(|_| invalid("schema"))?,
        mutating: true,
    })
}

/// Runs `add_workspace` on `view`.
pub(super) fn execute(
    view: &Entity<NativeView>,
    request: ApplicationCommandRequest,
    cx: &mut App,
) -> Result<ApplicationCommandResponse, BridgeError> {
    let ApplicationCommandRequest::Execute { name, arguments } = request else {
        return Err(invalid("request"));
    };
    let path = arguments
        .get("path")
        .and_then(|value| value.as_str())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| invalid("path"))?;
    if !view.update(cx, |view, cx| view.add_workspace(path, cx)) {
        return Err(BridgeError::new(
            ErrorCode::Busy,
            "A change blocks the window; try again when it ends.",
        ));
    }
    Ok(ApplicationCommandResponse::Result(
        ApplicationCommandResult {
            name,
            revision: None,
            output: r#"{"started":true}"#.parse().map_err(|_| invalid("output"))?,
        },
    ))
}

fn invalid(what: &str) -> BridgeError {
    BridgeError::new(
        ErrorCode::InvalidRequest,
        format!("Invalid add_workspace {what}."),
    )
}
