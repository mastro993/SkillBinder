//! The tools for skill updates from their sources and Sync with the library's Git remote.

use super::Server;
use super::call::Turn;
use super::inputs::{Agreed, SyncLibrary, UpdateSkills, Updating};
use super::results;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResponse;
use rmcp::{ErrorData, tool, tool_router};
use skillbinder_proto::command_line::Changed;

#[tool_router(router = updates_router, vis = "pub(super)")]
impl Server {
    #[tool(
        description = "Checks every skill's source for a newer version and lists the skills with one; updates skills, or all with an update, from their sources; or reinstalls a skill changed outside SkillBinder (drifted) from its record. Updated skills' copies are rewritten. list_skills filter updates lists them as of the last check.",
        output_schema = results::schema::<results::Updates>(),
        annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = false, open_world_hint = true)
    )]
    async fn update_skills(
        &self,
        Parameters(input): Parameters<UpdateSkills>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        match input.request() {
            Ok(Updating::Check(request)) => self.read(request, input.response_format, turn).await,
            Ok(Updating::Change(change)) => self.change(Ok(change), Agreed::default(), turn).await,
            Err(error) => Ok(results::error(&error.into()).into()),
        }
    }

    #[tool(
        description = "Syncs the library with its Git remote: sync pulls then pushes, push commits and pushes the library's changes, pull brings in the remote's; or sets or removes the remote.",
        output_schema = results::schema::<Changed>(),
        annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = false, open_world_hint = true)
    )]
    async fn sync_library(
        &self,
        Parameters(input): Parameters<SyncLibrary>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        let change = input.change();
        self.change(change, Agreed::default(), turn).await
    }
}
