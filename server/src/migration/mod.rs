mod m0001;

use axum::async_trait;
use futures::future::BoxFuture;
use sea_query::SchemaBuilder;
use sqlx::{
    migrate::{Migrate, Migration, MigrationSource, Migrator},
    Database, Pool,
};

// Migration sourcr for sea-query based migrations
struct SQSource<'a> {
    schema_builder: &'a (dyn SchemaBuilder + Send + Sync),
}

impl std::fmt::Debug for SQSource<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SQSource").finish()
    }
}

impl<'a> SQSource<'a> {
    fn new(schema_builder: &'a (dyn SchemaBuilder + Send + Sync)) -> Self {
        Self { schema_builder }
    }
}

impl<'s> MigrationSource<'s> for SQSource<'s> {
    fn resolve(self) -> BoxFuture<'s, Result<Vec<Migration>, sqlx::error::BoxDynError>> {
        Box::pin(async move { Ok(vec![m0001::into_migration(self.schema_builder)]) })
    }
}

pub async fn migrate<D: Database>(
    pool: &Pool<D>,
    schema_builder: &(dyn SchemaBuilder + Send + Sync),
) -> crate::Result<()>
where
    D::Connection: Migrate,
{
    let migration_source = SQSource::new(schema_builder);
    let migrator = Migrator::new(migration_source).await?;
    migrator.run(pool).await?;
    Ok(())
}
