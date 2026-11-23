//! The workspace and agent tools: the sidebar's workspaces and Settings › Agents.

use super::Server;
use super::call::Turn;
use super::inputs::{ChooseAgent, ManageWorkspace};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResponse;
use rmcp::{ErrorData, tool, tool_router};
use skillbinder_proto::command_line::Changed;

#[tool_router(router = places_router, vis = "pub(super)")]
impl Server {
    #[tool(
        description = "Adds the projects in a folder as workspaces, renames a workspace, or removes one from SkillBinder (its files and copies stay, unmanaged). Name a workspace by its name or by an absolute path inside it. Removing needs confirm, and replace answers for the user's own copies a new workspace meets; pass either only after the user agreed.",
        output_schema = super::results::schema::<Changed>(),
        annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = false, open_world_hint = false)
    )]
    async fn manage_workspace(
        &self,
        Parameters(input): Parameters<ManageWorkspace>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        let change = input.change();
        self.change(change, input.agreed(), turn).await
    }

    #[tool(
        description = "Turns an agent found on this computer on or off for SkillBinder. On, it becomes one of the user's agents and gets the active preset's skills; off, its copies stay in place, unmanaged. Pass replace only after the user agreed to replace their own copies.",
        output_schema = super::results::schema::<Changed>(),
        annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    async fn choose_agent(
        &self,
        Parameters(input): Parameters<ChooseAgent>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        let change = Ok(input.change());
        self.change(change, input.agreed(), turn).await
    }
}
