//! The MCP commands a native-test build advertises on its bridge.

use super::{holds, workspaces};
use gpui_kit::Entity;
use gpui_mcp::{
    ApplicationCommandRequest, ApplicationCommandResponse, BridgeError, BridgeHandle, ErrorCode,
    HostError,
};
use skillbinder_ui::NativeView;

/// Advertises `hold_activation` and `add_workspace` on the bridge, acting on `view`.
pub(crate) fn register_commands(
    bridge: &BridgeHandle,
    view: Entity<NativeView>,
) -> Result<(), HostError> {
    bridge.on_command(move |request, _, cx| match &request {
        ApplicationCommandRequest::List => Ok(ApplicationCommandResponse::List(vec![
            holds::descriptor()?,
            workspaces::descriptor()?,
        ])),
        ApplicationCommandRequest::Execute { name, .. } if name == holds::NAME => {
            holds::execute(&view, request, cx)
        }
        ApplicationCommandRequest::Execute { name, .. } if name == workspaces::NAME => {
            workspaces::execute(&view, request, cx)
        }
        ApplicationCommandRequest::Execute { .. } => {
            Err(BridgeError::new(ErrorCode::NotFound, "Unknown command."))
        }
    })
}
