//! Reviewed plans expire, but completed import outcomes remain replayable.
use super::support::Fixture;
use skillbinder_proto::{ErrorCategory, PlanId};

fn expire(data: &std::path::Path, plan: &PlanId) -> Result<(), Box<dyn std::error::Error>> {
    let database = rusqlite::Connection::open(data.join("state.sqlite3"))?;
    let payload: String = database.query_row(
        "SELECT payload FROM operation_plans WHERE id=?1",
        [plan.as_str()],
        |row| row.get(0),
    )?;
    let mut payload: serde_json::Value = serde_json::from_str(&payload)?;
    payload["review"]["expires_at"] = 0.into();
    database.execute(
        "UPDATE operation_plans SET payload=?1,expires_at=0 WHERE id=?2",
        rusqlite::params![serde_json::to_string(&payload)?, plan.as_str()],
    )?;
    Ok(())
}

#[tokio::test]
async fn expired_review_has_no_effect_and_completed_review_replays_after_expiry()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new();
    let source = fixture.skill("review", "Stable source");
    let source_bytes = std::fs::read(source.join("SKILL.md"))?;
    let engine = fixture.ready().await;
    fixture.register(&engine).await;
    let scan = fixture.scan(&engine).await;
    let ids = vec![scan.candidates[0].id.clone()];
    let expired = engine
        .prepare_import(scan.id.clone(), ids.clone(), false)
        .await?;
    let before = std::fs::read(fixture.data.join("library/.skillbinder.json"))?;
    expire(&fixture.data, &expired.id)?;
    assert!(
        engine
            .apply_import(expired.id)
            .await
            .is_err_and(|error| error.category == ErrorCategory::Stale)
    );
    assert_eq!(
        std::fs::read(fixture.data.join("library/.skillbinder.json"))?,
        before
    );
    let valid = engine.prepare_import(scan.id, ids, false).await?;
    let outcome = engine.apply_import(valid.id.clone()).await?;
    engine.shutdown().await?;
    expire(&fixture.data, &valid.id)?;
    let engine = fixture.open();
    assert_eq!(engine.apply_import(valid.id).await?, outcome);
    assert_eq!(std::fs::read(source.join("SKILL.md"))?, source_bytes);
    engine.shutdown().await?;
    Ok(())
}
