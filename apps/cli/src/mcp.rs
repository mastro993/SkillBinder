//! `skillbinder mcp` (ADR 0025): the command's actions as MCP tools over stdio. Each tool call
//! becomes the request the command sends, over the same socket and under the same rules; the
//! server touches the app only once a tool is called (spec rule 2).

mod call;
mod inputs;
mod results;
mod session;
#[cfg(test)]
mod tests;

use call::Turn;
use inputs::{
    GetSkill, ListPlaces, ListSkills, OrganizeLibrary, ResolveCopies, SetSkillsActive, Status,
    SwitchPreset,
};
use results::schema;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CacheScope, CallToolResponse, Implementation, ListToolsResult, PaginatedRequestParams,
    ProtocolVersion, ServerCapabilities, ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::{
    ErrorData, RoleServer, ServerHandler, ServiceExt as _, tool, tool_handler, tool_router,
};
use session::App;
use skillbinder_proto::command_line::{Changed, Request};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

/// What the server tells agents before they call anything (spec rule 8).
const INSTRUCTIONS: &str = "SkillBinder keeps the user's library of agent skills and writes \
copies of the active ones into their agents' skill directories. Read before you change: call \
status, list_skills, or list_places first. Pass replace or confirm only after the user said \
so; when a result says needs_answer, ask the user. Changes show in SkillBinder's window as they \
run.";

/// Serves MCP on stdin and stdout until the client goes; `data` is the app's application data,
/// passed on to an app it starts when `isolated`.
pub(crate) fn serve(data: PathBuf, isolated: bool) -> ExitCode {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("skillbinder mcp can't start: {error}");
            return ExitCode::FAILURE;
        }
    };
    let server = Server::new(App::new(data, isolated));
    let served = runtime.block_on(async {
        let running = server
            .serve(rmcp::transport::stdio())
            .await
            .map_err(|error| error.to_string())?;
        running
            .waiting()
            .await
            .map(drop)
            .map_err(|error| error.to_string())
    });
    match served {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("skillbinder mcp stopped: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The MCP server: one app, shared by every call.
#[derive(Clone)]
pub(crate) struct Server {
    app: Arc<App>,
}

impl Server {
    fn new(app: App) -> Self {
        Self { app: Arc::new(app) }
    }
}

#[tool_router]
impl Server {
    #[tool(
        description = "SkillBinder's version, whether setup finished, the active preset, the library's counts, copies waiting for the user's answer, Sync's state, and what runs now.",
        output_schema = schema::<skillbinder_proto::command_line::StatusOutput>(),
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = false)
    )]
    async fn status(
        &self,
        Parameters(input): Parameters<Status>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        self.read(Request::Status, input.response_format, turn)
            .await
    }

    #[tool(
        description = "The library's skills, each with where it is active (agents and workspaces) and its marks: update, drifted, new. Filter by all, active, inactive, or updates, and by library folder.",
        output_schema = schema::<results::Skills>(),
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = false)
    )]
    async fn list_skills(
        &self,
        Parameters(input): Parameters<ListSkills>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        self.read(input.request(), input.response_format, turn)
            .await
    }

    #[tool(
        description = "One library skill: its description, source, presets, where it is active, and its copies in skill directories.",
        output_schema = schema::<skillbinder_proto::command_line::SkillOutput>(),
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = false)
    )]
    async fn get_skill(
        &self,
        Parameters(input): Parameters<GetSkill>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        self.read(input.request(), input.response_format, turn)
            .await
    }

    #[tool(
        description = "Lists one kind of place: presets (marking the active one), library folders, or copies waiting for the user's answer.",
        output_schema = schema::<results::Places>(),
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = false)
    )]
    async fn list_places(
        &self,
        Parameters(input): Parameters<ListPlaces>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        self.read(input.request(), input.response_format, turn)
            .await
    }

    #[tool(
        description = "Turns skills on or off for the user's agents in the active preset, or adds them to or removes them from another preset. Writing copies can meet the user's own copy of a skill; pass replace only after the user agreed to replace it.",
        output_schema = schema::<Changed>(),
        annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    async fn set_skills_active(
        &self,
        Parameters(input): Parameters<SetSkillsActive>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        let change = input.change();
        self.change(change, input.agreed(), turn).await
    }

    #[tool(
        description = "Makes a preset active, writing its skills for the user's agents and removing the previous preset's. The switch shows in SkillBinder's window. Pass replace only after the user agreed to replace their own copies.",
        output_schema = schema::<Changed>(),
        annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    async fn switch_preset(
        &self,
        Parameters(input): Parameters<SwitchPreset>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        let change = Ok(input.change());
        self.change(change, input.agreed(), turn).await
    }

    #[tool(
        description = "Creates, renames, clones, or deletes a preset; creates, renames, or deletes a library folder; or files skills in a folder. Deleting needs confirm, passed only after the user agreed; skills always stay in the library.",
        output_schema = schema::<Changed>(),
        annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = false, open_world_hint = false)
    )]
    async fn organize_library(
        &self,
        Parameters(input): Parameters<OrganizeLibrary>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        let change = input.change();
        self.change(change, input.agreed(), turn).await
    }

    #[tool(
        description = "Answers for copies waiting for the user (list_places kind copies): replace the user's copy with the library's, keep theirs, or retry copies that couldn't be written or removed. Call it only with the user's answer.",
        output_schema = schema::<Changed>(),
        annotations(read_only_hint = false, destructive_hint = true, idempotent_hint = false, open_world_hint = false)
    )]
    async fn resolve_copies(
        &self,
        Parameters(input): Parameters<ResolveCopies>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        let changes = input.changes();
        self.changes(changes, turn).await
    }
}

/// The tools in the order they list (spec Tools), rather than the router's by name.
const ORDER: [&str; 8] = [
    "status",
    "list_skills",
    "get_skill",
    "list_places",
    "set_skills_active",
    "switch_preset",
    "organize_library",
    "resolve_copies",
];

#[tool_handler]
impl ServerHandler for Server {
    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let mut tools = Self::tool_router().list_all();
        tools.sort_by_key(|tool| ORDER.iter().position(|name| *name == tool.name));
        let mut listed = ListToolsResult::with_all_items(tools);
        // Protocol 2026-07-28 asks for cache hints; the list never changes while the server runs.
        if context
            .protocol_version()
            .is_some_and(|version| version >= ProtocolVersion::V_2026_07_28)
        {
            listed.ttl_ms = Some(0);
            listed.cache_scope = Some(CacheScope::Public);
        }
        Ok(listed)
    }

    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "skillbinder",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(INSTRUCTIONS)
    }
}
