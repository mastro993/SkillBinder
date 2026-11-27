//! The tools for skill updates from their sources and Sync with the library's Git remote.

use super::Server;
use super::call::Turn;
use super::inputs::{Agreed, ResponseFormat, Sending, SyncLibrary, UpdateSkills};
use super::results;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResponse;
use rmcp::{ErrorData, tool, tool_router};

#[tool_router(router = updates_router, vis = "pub(super)")]
impl Server {
    #[tool(
        description = "Checks every skill's source for a newer version and lists the skills with one; diff shows what updating skills would change, as a patch for each, changing nothing; updates skills, or all with an update, from their sources; or reinstalls a skill changed outside SkillBinder (drifted) from its record. Updated skills' copies are rewritten. list_skills filter updates lists them as of the last check.",
        output_schema = results::schema::<results::Updates>(),
        annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = false, open_world_hint = true)
    )]
    async fn update_skills(
        &self,
        Parameters(input): Parameters<UpdateSkills>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        match input.request() {
            Ok(Sending::Read(request)) => self.read(request, input.response_format, turn).await,
            Ok(Sending::Change(change)) => self.change(Ok(change), Agreed::default(), turn).await,
            Err(error) => Ok(results::error(&error.into()).into()),
        }
    }

    #[tool(
        description = "Syncs the library with its Git remote: sync pulls then pushes, push commits and pushes the library's changes, pull brings in the remote's; or sets or removes the remote. status lists the skills not synced yet on each side and those changed on both, which sync refuses until resolve keeps mine or theirs of each. history lists the library's changes with the device that made each; restore puts a skill back to its version in one, and discard undoes this device's change not pushed yet. rename_device renames this device for the changes it pushes.",
        output_schema = results::schema::<results::Synced>(),
        annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = false, open_world_hint = true)
    )]
    async fn sync_library(
        &self,
        Parameters(input): Parameters<SyncLibrary>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        match input.request() {
            Ok(Sending::Read(request)) => self.read(request, ResponseFormat::default(), turn).await,
            Ok(Sending::Change(change)) => self.change(Ok(change), Agreed::default(), turn).await,
            Err(error) => Ok(results::error(&error.into()).into()),
        }
    }
}
