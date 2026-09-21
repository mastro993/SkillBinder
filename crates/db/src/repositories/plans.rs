//! Import plans and the idempotency records that make a retried import safe.
//!
//! The plan payload and the recorded result are JSON text produced by the caller.

use crate::schema::{idempotency_records, operation_plans};
use diesel::SqliteConnection;
use diesel::prelude::*;

pub(crate) fn create(
    connection: &mut SqliteConnection,
    id: &str,
    payload: &str,
    expires_at: i64,
) -> QueryResult<()> {
    diesel::insert_into(operation_plans::table)
        .values((
            operation_plans::id.eq(id),
            operation_plans::payload.eq(payload),
            operation_plans::expires_at.eq(expires_at),
        ))
        .execute(connection)?;
    Ok(())
}

/// The payload of an unconsumed plan, or `None` when the plan is unknown or already consumed.
pub(crate) fn load(connection: &mut SqliteConnection, id: &str) -> QueryResult<Option<String>> {
    operation_plans::table
        .select(operation_plans::payload)
        .filter(operation_plans::id.eq(id))
        .filter(operation_plans::consumed.eq(0))
        .first(connection)
        .optional()
}

/// Marks a plan consumed. Consuming a plan twice is a no-op.
pub(crate) fn consume(connection: &mut SqliteConnection, id: &str) -> QueryResult<()> {
    diesel::update(
        operation_plans::table
            .filter(operation_plans::id.eq(id))
            .filter(operation_plans::consumed.eq(0)),
    )
    .set(operation_plans::consumed.eq(1))
    .execute(connection)?;
    Ok(())
}

/// Drops unconsumed plans that expired before `now`.
pub(crate) fn expire(connection: &mut SqliteConnection, now: i64) -> QueryResult<()> {
    diesel::delete(
        operation_plans::table
            .filter(operation_plans::expires_at.lt(now))
            .filter(operation_plans::consumed.eq(0)),
    )
    .execute(connection)?;
    Ok(())
}

/// The recorded `(request_hash, result)` for an operation, if it was already answered.
pub(crate) fn idempotency_lookup(
    connection: &mut SqliteConnection,
    operation_id: &str,
) -> QueryResult<Option<(String, String)>> {
    idempotency_records::table
        .select((
            idempotency_records::request_hash,
            idempotency_records::result,
        ))
        .filter(idempotency_records::operation_id.eq(operation_id))
        .first(connection)
        .optional()
}

/// Records the answer to an operation; the first answer wins.
pub(crate) fn idempotency_store(
    connection: &mut SqliteConnection,
    id: &str,
    request_hash: &str,
    result: &str,
) -> QueryResult<()> {
    diesel::insert_into(idempotency_records::table)
        .values((
            idempotency_records::operation_id.eq(id),
            idempotency_records::request_hash.eq(request_hash),
            idempotency_records::result.eq(result),
        ))
        .on_conflict(idempotency_records::operation_id)
        .do_nothing()
        .execute(connection)?;
    Ok(())
}

/// Consuming the plan and recording its result happen together or not at all.
pub(crate) fn consume_and_store(
    connection: &mut SqliteConnection,
    id: &str,
    request_hash: &str,
    result: &str,
) -> QueryResult<()> {
    connection.transaction(|connection| {
        consume(connection, id)?;
        idempotency_store(connection, id, request_hash, result)
    })
}
