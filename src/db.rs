use crate::config::RemoteDatabaseSettings;
use crate::error::Result;
use sea_query::{Alias, Expr, Iden, PostgresQueryBuilder, Query, SimpleExpr, SqliteQueryBuilder};
use sea_query_binder::SqlxBinder;
use sqlx::{migrate::Migrator, Executor, PgPool, Pool, Postgres, Sqlite, SqlitePool, Transaction};
use std::path::Path;
use tracing::{debug, info, instrument, warn};

static SQLITE_MIGRATOR: Migrator = sqlx::migrate!("./migrations/sqlite");
static POSTGRES_MIGRATOR: Migrator = sqlx::migrate!("./migrations/postgres");

#[derive(Iden)]
#[iden = "metrics"]
enum MetricsIden {
    Table,
    #[allow(dead_code)]
    Id,
    Keypresses,
    MouseClicks,
    MouseDistanceIn,
    MouseDistanceMi,
    ScrollSteps,
    #[allow(dead_code)]
    Timestamp,
}

#[derive(Iden)]
#[iden = "metrics_summary"]
enum MetricsSummaryIden {
    Table,
    Id,
    LastUpdated,
    TotalKeypresses,
    TotalMouseClicks,
    TotalMouseTravelIn,
    TotalMouseTravelMi,
    TotalScrollSteps,
}

#[derive(Debug, Clone)]
pub struct MetricsData {
    pub keypresses: usize,
    pub mouse_clicks: usize,
    pub scroll_steps: usize,
    pub mouse_distance_in: f64,
}

#[instrument(skip(remote_settings))]
pub async fn setup_database_pools(
    local_db_path: &str,
    remote_settings: &RemoteDatabaseSettings,
) -> Result<(Pool<Sqlite>, Option<Pool<Postgres>>)> {
    info!("Setting up database pools...");

    info!("Setting up local SQLite pool at: {}", local_db_path);
    if let Some(parent_dir) = Path::new(local_db_path).parent() {
        tokio::fs::create_dir_all(parent_dir).await?;
    }
    let sqlite_pool = SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(local_db_path)
            .create_if_missing(true),
    )
    .await?;
    info!("Local SQLite pool created.");

    let pg_pool_option: Option<Pool<Postgres>> = match &remote_settings.postgres_url {
        Some(url) if !url.is_empty() => {
            info!("Setting up remote Postgres pool for URL...");
            match PgPool::connect(url).await {
                Ok(pool) => {
                    info!("Remote Postgres pool created.");
                    Some(pool)
                }
                Err(e) => {
                    warn!("Failed to connect to remote Postgres DB: {}. Remote sync will be disabled.", e);
                    None
                }
            }
        }
        _ => {
            info!("No remote Postgres URL configured.");
            None
        }
    };

    Ok((sqlite_pool, pg_pool_option))
}

#[instrument(skip(sqlite_pool, pg_pool_option))]
pub async fn run_migrations(
    sqlite_pool: &Pool<Sqlite>,
    pg_pool_option: &Option<Pool<Postgres>>,
) -> Result<()> {
    info!("Running database migrations...");

    info!("Running migrations on local SQLite DB...");
    SQLITE_MIGRATOR.run(sqlite_pool).await?;
    info!("Local SQLite migrations completed.");

    if let Some(pg_pool) = pg_pool_option {
        info!("Running migrations on remote Postgres DB...");
        match POSTGRES_MIGRATOR.run(pg_pool).await {
            Ok(_) => info!("Remote Postgres migrations completed."),
            Err(e) => {
                warn!(
                    "Failed to run migrations on remote Postgres DB: {}. Remote sync might fail.",
                    e
                );
            }
        }
    }
    Ok(())
}

#[instrument(skip(pool))]
pub async fn load_initial_totals(pool: &Pool<Sqlite>) -> Result<(usize, usize, usize, f64)> {
    // First try loading from the summary table
    match load_initial_totals_from_summary(pool).await {
        Ok(totals) => Ok(totals),
        Err(e) => {
            warn!("Failed to load totals from summary table: {}. Falling back to aggregating metrics table.", e);
            load_initial_totals_from_metrics(pool).await
        }
    }
}

#[instrument(skip(pool))]
async fn load_initial_totals_from_metrics(
    pool: &Pool<Sqlite>,
) -> Result<(usize, usize, usize, f64)> {
    info!("Loading initial totals by aggregating metrics table...");
    let query = Query::select()
        .expr_as(
            Expr::col(MetricsIden::Keypresses).sum(),
            Alias::new("total_keys"),
        )
        .expr_as(
            Expr::col(MetricsIden::MouseClicks).sum(),
            Alias::new("total_clicks"),
        )
        .expr_as(
            Expr::col(MetricsIden::ScrollSteps).sum(),
            Alias::new("total_scrolls"),
        )
        .expr_as(
            Expr::col(MetricsIden::MouseDistanceIn).sum(),
            Alias::new("total_distance"),
        )
        .from(MetricsIden::Table)
        .to_owned();

    let (sql, values) = query.build_sqlx(SqliteQueryBuilder);

    let row_opt = sqlx::query_with(&sql, values).fetch_optional(pool).await?;

    match row_opt {
        Some(r) => {
            use sqlx::Row;
            let keys: i64 = r.try_get("total_keys").unwrap_or(0);
            let clicks: i64 = r.try_get("total_clicks").unwrap_or(0);
            let scrolls: i64 = r.try_get("total_scrolls").unwrap_or(0);
            let distance: f64 = r.try_get("total_distance").unwrap_or(0.0);
            info!(
                "Loaded totals from metrics table: K={}, C={}, S={}, D={:.2}",
                keys, clicks, scrolls, distance
            );
            Ok((keys as usize, clicks as usize, scrolls as usize, distance))
        }
        None => {
            info!("No previous data found in metrics table, starting totals from zero.");
            Ok((0, 0, 0, 0.0))
        }
    }
}

#[instrument(skip(pool))]
async fn load_initial_totals_from_summary(
    pool: &Pool<Sqlite>,
) -> Result<(usize, usize, usize, f64)> {
    info!("Loading initial totals from metrics_summary table...");
    let query = Query::select()
        .columns([
            MetricsSummaryIden::TotalKeypresses,
            MetricsSummaryIden::TotalMouseClicks,
            MetricsSummaryIden::TotalScrollSteps,
            MetricsSummaryIden::TotalMouseTravelIn,
        ])
        .from(MetricsSummaryIden::Table)
        .and_where(Expr::col(MetricsSummaryIden::Id).eq(1))
        .limit(1)
        .to_owned();

    let (sql, values) = query.build_sqlx(SqliteQueryBuilder);

    let row_opt = sqlx::query_with(&sql, values).fetch_optional(pool).await?;

    match row_opt {
        Some(r) => {
            use sqlx::Row;
            let keys: i64 = r.try_get(MetricsSummaryIden::TotalKeypresses.to_string().as_str())?;
            let clicks: i64 =
                r.try_get(MetricsSummaryIden::TotalMouseClicks.to_string().as_str())?;
            let scrolls: i64 =
                r.try_get(MetricsSummaryIden::TotalScrollSteps.to_string().as_str())?;
            let distance: f64 =
                r.try_get(MetricsSummaryIden::TotalMouseTravelIn.to_string().as_str())?;
            info!(
                "Loaded totals from summary: K={}, C={}, S={}, D={:.2}",
                keys, clicks, scrolls, distance
            );
            Ok((keys as usize, clicks as usize, scrolls as usize, distance))
        }
        None => {
            warn!("Metrics summary row (ID=1) not found! Initializing totals to zero. Please check migrations.");
            Ok((0, 0, 0, 0.0))
        }
    }
}

async fn update_summary_table_sqlite<'c, E>(executor: E, data: &MetricsData) -> Result<()>
where
    E: Executor<'c, Database = sqlx::Sqlite>,
{
    let distance_in = data.mouse_distance_in;
    let distance_mi = distance_in / 63360.0;

    let values_to_update = vec![
        (
            MetricsSummaryIden::TotalKeypresses,
            Expr::col(MetricsSummaryIden::TotalKeypresses)
                .add(data.keypresses as i64)
                .into(),
        ),
        (
            MetricsSummaryIden::TotalMouseClicks,
            Expr::col(MetricsSummaryIden::TotalMouseClicks)
                .add(data.mouse_clicks as i64)
                .into(),
        ),
        (
            MetricsSummaryIden::TotalScrollSteps,
            Expr::col(MetricsSummaryIden::TotalScrollSteps)
                .add(data.scroll_steps as i64)
                .into(),
        ),
        (
            MetricsSummaryIden::TotalMouseTravelIn,
            Expr::col(MetricsSummaryIden::TotalMouseTravelIn)
                .add(distance_in)
                .into(),
        ),
        (
            MetricsSummaryIden::TotalMouseTravelMi,
            Expr::col(MetricsSummaryIden::TotalMouseTravelMi)
                .add(distance_mi)
                .into(),
        ),
        (
            MetricsSummaryIden::LastUpdated,
            SimpleExpr::Custom("CURRENT_TIMESTAMP".into()),
        ),
    ];

    let query = Query::update()
        .table(MetricsSummaryIden::Table)
        .values(values_to_update)
        .and_where(Expr::col(MetricsSummaryIden::Id).eq(1))
        .to_owned();

    let (sql, values) = query.build_sqlx(SqliteQueryBuilder);

    let rows_affected = sqlx::query_with(&sql, values)
        .execute(executor)
        .await?
        .rows_affected();

    if rows_affected != 1 {
        warn!(
            "SQLite metrics summary update affected {} rows, expected 1. Summary might be incorrect.",
            rows_affected
        );
    }

    Ok(())
}

async fn update_summary_table_postgres<'c, E>(executor: E, data: &MetricsData) -> Result<()>
where
    E: Executor<'c, Database = sqlx::Postgres>,
{
    let distance_in = data.mouse_distance_in;
    let distance_mi = distance_in / 63360.0;

    let values_to_update = vec![
        (
            MetricsSummaryIden::TotalKeypresses,
            Expr::col(MetricsSummaryIden::TotalKeypresses)
                .add(data.keypresses as i64)
                .into(),
        ),
        (
            MetricsSummaryIden::TotalMouseClicks,
            Expr::col(MetricsSummaryIden::TotalMouseClicks)
                .add(data.mouse_clicks as i64)
                .into(),
        ),
        (
            MetricsSummaryIden::TotalScrollSteps,
            Expr::col(MetricsSummaryIden::TotalScrollSteps)
                .add(data.scroll_steps as i64)
                .into(),
        ),
        (
            MetricsSummaryIden::TotalMouseTravelIn,
            Expr::col(MetricsSummaryIden::TotalMouseTravelIn)
                .add(distance_in)
                .into(),
        ),
        (
            MetricsSummaryIden::TotalMouseTravelMi,
            Expr::col(MetricsSummaryIden::TotalMouseTravelMi)
                .add(distance_mi)
                .into(),
        ),
        (
            MetricsSummaryIden::LastUpdated,
            SimpleExpr::Custom("CURRENT_TIMESTAMP".into()),
        ),
    ];

    let query = Query::update()
        .table(MetricsSummaryIden::Table)
        .values(values_to_update)
        .and_where(Expr::col(MetricsSummaryIden::Id).eq(1))
        .to_owned();

    let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

    let rows_affected = sqlx::query_with(&sql, values)
        .execute(executor)
        .await?
        .rows_affected();

    if rows_affected != 1 {
        warn!(
            "Postgres metrics summary update affected {} rows, expected 1. Summary might be incorrect.",
            rows_affected
        );
    }

    Ok(())
}

async fn persist_metrics_sqlite_in_tx(
    tx: &mut Transaction<'_, Sqlite>,
    data: &MetricsData,
) -> Result<()> {
    let distance_mi = data.mouse_distance_in / 63360.0;

    let mut query_metrics = Query::insert();
    query_metrics
        .into_table(MetricsIden::Table)
        .columns([
            MetricsIden::Keypresses,
            MetricsIden::MouseClicks,
            MetricsIden::ScrollSteps,
            MetricsIden::MouseDistanceIn,
            MetricsIden::MouseDistanceMi,
        ])
        .values_panic([
            (data.keypresses as i64).into(),
            (data.mouse_clicks as i64).into(),
            (data.scroll_steps as i64).into(),
            data.mouse_distance_in.into(),
            distance_mi.into(),
        ]);
    let (sql_metrics, values_metrics) = query_metrics.build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql_metrics, values_metrics)
        .execute(&mut **tx)
        .await?;

    update_summary_table_sqlite(&mut **tx, data).await?;

    Ok(())
}

async fn persist_metrics_postgres_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    data: &MetricsData,
) -> Result<()> {
    let distance_mi = data.mouse_distance_in / 63360.0;

    let mut query_metrics = Query::insert();
    query_metrics
        .into_table(MetricsIden::Table)
        .columns([
            MetricsIden::Keypresses,
            MetricsIden::MouseClicks,
            MetricsIden::ScrollSteps,
            MetricsIden::MouseDistanceIn,
            MetricsIden::MouseDistanceMi,
        ])
        .values_panic([
            (data.keypresses as i64).into(),
            (data.mouse_clicks as i64).into(),
            (data.scroll_steps as i64).into(),
            data.mouse_distance_in.into(),
            distance_mi.into(),
        ]);
    let (sql_metrics, values_metrics) = query_metrics.build_sqlx(PostgresQueryBuilder);
    sqlx::query_with(&sql_metrics, values_metrics)
        .execute(&mut **tx)
        .await?;

    update_summary_table_postgres(&mut **tx, data).await?;

    Ok(())
}

#[instrument(skip(pool, data), fields(db_type = "sqlite"))]
pub async fn persist_metrics_sqlite(pool: &Pool<Sqlite>, data: &MetricsData) -> Result<()> {
    if data.keypresses == 0
        && data.mouse_clicks == 0
        && data.scroll_steps == 0
        && data.mouse_distance_in == 0.0
    {
        return Ok(());
    }

    persist_metrics_transactional_sqlite(pool, data).await
}

#[instrument(skip(pool, data), fields(db_type = "postgres"))]
pub async fn persist_metrics_postgres(pool: &Pool<Postgres>, data: &MetricsData) -> Result<()> {
    if data.keypresses == 0
        && data.mouse_clicks == 0
        && data.scroll_steps == 0
        && data.mouse_distance_in == 0.0
    {
        return Ok(());
    }

    persist_metrics_transactional_postgres(pool, data).await
}

#[instrument(skip(pool, data), fields(db_type = "sqlite"))]
pub async fn persist_metrics_transactional_sqlite(
    pool: &Pool<Sqlite>,
    data: &MetricsData,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    let result = persist_metrics_sqlite_in_tx(&mut tx, data).await;
    
    match result {
        Ok(_) => {
            tx.commit().await?;
            debug!(
                "SQLite transaction committed for metrics interval: {:?}",
                data
            );
            Ok(())
        }
        Err(e) => {
            warn!("SQLite transaction failed, rolling back: {}", e);
            let _ = tx.rollback().await;
            Err(e)
        }
    }
}

#[instrument(skip(pool, data), fields(db_type = "postgres"))]
pub async fn persist_metrics_transactional_postgres(
    pool: &Pool<Postgres>,
    data: &MetricsData,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    let result = persist_metrics_postgres_in_tx(&mut tx, data).await;
    
    match result {
        Ok(_) => {
            tx.commit().await?;
            debug!(
                "Postgres transaction committed for metrics interval: {:?}",
                data
            );
            Ok(())
        }
        Err(e) => {
            warn!("Postgres transaction failed, rolling back: {}", e);
            let _ = tx.rollback().await;
            Err(e)
        }
    }
}
