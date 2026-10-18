//! Adds the projects in a picked folder as workspaces.
use skillbinder_engine::Engine;

pub async fn add_folder(
    engine: &Engine,
    folder: &std::path::Path,
) -> skillbinder_proto::AppResult<Vec<skillbinder_proto::Workspace>> {
    let grant = engine.grant_directory(folder.to_owned()).await.unwrap();
    engine.add_workspaces(grant.id).await
}
