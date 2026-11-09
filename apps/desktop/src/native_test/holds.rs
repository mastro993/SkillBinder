//! The `hold_activation` MCP command: a parity capture holds the progress strip or the end toast
//! so a state that real timing passes too fast to catch stays on screen.

use gpui_kit::Entity;
use gpui_mcp::{
    ApplicationCommandDescriptor, ApplicationCommandRequest, ApplicationCommandResponse,
    ApplicationCommandResult, BridgeError, BridgeHandle, ErrorCode, HostError,
};
use skillbinder_ui::{NativeView, StripHold};

const NAME: &str = "hold_activation";

const SCHEMA: &str = r#"{
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "strip": {
      "description": "null releases the strip. \"hidden\" keeps a strip that starts or is up unrevealed: only the transparent input blocker, for the wash formula's base capture. An integer 0-100 shows that percent once the strip reveals. A held strip never ends; releasing ends one whose change has landed.",
      "anyOf": [
        { "type": "null" },
        { "const": "hidden" },
        { "type": "integer", "minimum": 0, "maximum": 100 }
      ]
    },
    "toast": {
      "type": "boolean",
      "description": "true keeps the end toast up until a call with false, which gives it 1.5 s more."
    }
  }
}"#;

/// Advertises `hold_activation` on the bridge, acting on `view`.
pub(crate) fn register_holds(
    bridge: &BridgeHandle,
    view: Entity<NativeView>,
) -> Result<(), HostError> {
    bridge.on_command(move |request, _, cx| match request {
        ApplicationCommandRequest::List => Ok(ApplicationCommandResponse::List(vec![
            ApplicationCommandDescriptor {
                name: NAME.to_owned(),
                title: "Hold activation progress".to_owned(),
                description: "Holds or releases the blocking progress strip and its end toast \
                              for a native capture."
                    .to_owned(),
                input_schema: SCHEMA.parse().map_err(|_| invalid("schema"))?,
                mutating: true,
            },
        ])),
        ApplicationCommandRequest::Execute { name, arguments } => {
            if name != NAME {
                return Err(BridgeError::new(ErrorCode::NotFound, "Unknown command."));
            }
            let strip = match arguments.get("strip") {
                None => None,
                Some(value) if value.is_null() => None,
                Some(value) if value.as_str() == Some("hidden") => Some(StripHold::Hidden),
                Some(value) => {
                    let percent = value
                        .as_u64()
                        .and_then(|percent| u8::try_from(percent).ok())
                        .filter(|percent| *percent <= 100)
                        .ok_or_else(|| invalid("strip"))?;
                    Some(StripHold::Percent(percent))
                }
            };
            let toast = match arguments.get("toast") {
                None => false,
                Some(value) => value.as_bool().ok_or_else(|| invalid("toast"))?,
            };
            view.update(cx, |view, cx| view.hold_activation(strip, toast, cx));
            let shown = match strip {
                None => "null".to_owned(),
                Some(StripHold::Hidden) => "\"hidden\"".to_owned(),
                Some(StripHold::Percent(percent)) => percent.to_string(),
            };
            let output = format!(r#"{{"strip":{shown},"toast":{toast}}}"#);
            Ok(ApplicationCommandResponse::Result(
                ApplicationCommandResult {
                    name,
                    revision: None,
                    output: output.parse().map_err(|_| invalid("output"))?,
                },
            ))
        }
    })
}

fn invalid(what: &str) -> BridgeError {
    BridgeError::new(
        ErrorCode::InvalidRequest,
        format!("Invalid hold_activation {what}."),
    )
}
