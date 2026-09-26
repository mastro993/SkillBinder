use crate::schema::{binding_actions, binding_receipts};
use diesel::{SqliteConnection, prelude::*};

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = binding_actions)]
pub struct BindingActionRow {
    pub id: String,
    pub skill_ids: String,
    pub scope: String,
    pub project_root_id: Option<String>,
    pub agent_ids: String,
    pub target_paths: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = binding_receipts)]
pub struct BindingReceiptRow {
    pub target_path: String,
    pub skill_id: String,
    pub digest: String,
}

pub fn list(connection: &mut SqliteConnection) -> QueryResult<Vec<BindingActionRow>> {
    binding_actions::table
        .select(BindingActionRow::as_select())
        .order((
            binding_actions::created_at.desc(),
            binding_actions::id.asc(),
        ))
        .load(connection)
}

pub fn insert(connection: &mut SqliteConnection, row: &BindingActionRow) -> QueryResult<()> {
    diesel::insert_into(binding_actions::table)
        .values((
            binding_actions::id.eq(&row.id),
            binding_actions::skill_ids.eq(&row.skill_ids),
            binding_actions::scope.eq(&row.scope),
            binding_actions::project_root_id.eq(&row.project_root_id),
            binding_actions::agent_ids.eq(&row.agent_ids),
            binding_actions::target_paths.eq(&row.target_paths),
            binding_actions::created_at.eq(row.created_at),
        ))
        .execute(connection)?;
    Ok(())
}

pub fn receipt(
    connection: &mut SqliteConnection,
    path: &str,
) -> QueryResult<Option<BindingReceiptRow>> {
    binding_receipts::table
        .filter(binding_receipts::target_path.eq(path))
        .select(BindingReceiptRow::as_select())
        .first(connection)
        .optional()
}

pub fn put_receipt(connection: &mut SqliteConnection, row: &BindingReceiptRow) -> QueryResult<()> {
    diesel::insert_into(binding_receipts::table)
        .values((
            binding_receipts::target_path.eq(&row.target_path),
            binding_receipts::skill_id.eq(&row.skill_id),
            binding_receipts::digest.eq(&row.digest),
        ))
        .on_conflict(binding_receipts::target_path)
        .do_update()
        .set((
            binding_receipts::skill_id.eq(&row.skill_id),
            binding_receipts::digest.eq(&row.digest),
        ))
        .execute(connection)?;
    Ok(())
}
