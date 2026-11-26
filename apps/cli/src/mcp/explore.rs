//! The tools that bring skills into the library: Explore's search and Install, Scan for skills
//! with its import and ignored skills, and the user's repositories.

use super::Server;
use super::call::Turn;
use super::inputs::{
    Agreed, FindSkills, IgnoreSkills, ImportSkills, InstallSkills, ManageRepositories,
};
use super::results;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResponse;
use rmcp::{ErrorData, tool, tool_router};
use skillbinder_proto::command_line::Changed;

#[tool_router(router = explore_router, vis = "pub(super)")]
impl Server {
    #[tool(
        description = "Finds skills to bring into the library. where explore searches the catalog, the user's repositories, and skills.sh for query, each result with the reference install_skills takes; where found scans the user's agents and workspaces now and lists skills found there that are not in the library and not ignored, with their versions, for import_skills, then every ignored slug.",
        output_schema = results::schema::<results::Finds>(),
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    async fn find_skills(
        &self,
        Parameters(input): Parameters<FindSkills>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        match input.request() {
            Ok(request) => self.read(request, input.response_format, turn).await,
            Err(error) => Ok(results::error(&error.into()).into()),
        }
    }

    #[tool(
        description = "Installs skills from Explore into the library, by the references find_skills gives. They arrive inactive: turn them on with set_skills_active.",
        output_schema = results::schema::<Changed>(),
        annotations(read_only_hint = false, destructive_hint = false, idempotent_hint = false, open_world_hint = true)
    )]
    async fn install_skills(
        &self,
        Parameters(input): Parameters<InstallSkills>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        let change = input.change();
        self.change(change, Agreed::default(), turn).await
    }

    #[tool(
        description = "Imports skills find_skills found (where found) into the library, each staying active where it was found: by slug, with the version when one has several, or all the Import skills dialog ticks. An import that replaces the user's copies says so first; pass replace only after the user agreed.",
        output_schema = results::schema::<Changed>(),
        annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = false, open_world_hint = false)
    )]
    async fn import_skills(
        &self,
        Parameters(input): Parameters<ImportSkills>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        let change = input.change();
        self.change(change, input.agreed(), turn).await
    }

    #[tool(
        description = "Ignores found skills (find_skills where found) by slug, so they are no longer offered for import, nor imported by import_skills all; with stop, offers ignored ones again. Changes no file. Importing a skill by slug ends its ignore.",
        output_schema = results::schema::<Changed>(),
        annotations(read_only_hint = false, destructive_hint = false, idempotent_hint = true, open_world_hint = false)
    )]
    async fn ignore_skills(
        &self,
        Parameters(input): Parameters<IgnoreSkills>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        let change = input.change();
        self.change(change, Agreed::default(), turn).await
    }

    #[tool(
        description = "Adds a Git repository to browse and install its skills, removes one (its installed skills stay in the library), or installs each of its skills the library lacks, inactive. list_places kind repositories lists them.",
        output_schema = results::schema::<Changed>(),
        annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = false, open_world_hint = true)
    )]
    async fn manage_repositories(
        &self,
        Parameters(input): Parameters<ManageRepositories>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        let change = Ok(input.change());
        self.change(change, Agreed::default(), turn).await
    }
}
