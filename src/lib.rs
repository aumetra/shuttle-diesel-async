use async_trait::async_trait;
use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
    AsyncConnection, AsyncPgConnection,
};
use diesel_migrations_async::{EmbeddedMigrations, MigrationHarness};
use serde::Serialize;
use shuttle_service::{
    database::{SharedEngine, Type as DatabaseType},
    Factory, ResourceBuilder, Type,
};

pub use diesel_async;
pub use diesel_migrations_async;

#[derive(Default, Serialize)]
pub struct Postgres {
    #[serde(skip)]
    migrations: Option<EmbeddedMigrations>,
    pool_size: Option<usize>,
}

impl Postgres {
    pub fn migrations(self, migrations: EmbeddedMigrations) -> Self {
        Self {
            migrations: Some(migrations),
            ..self
        }
    }

    pub fn pool_size(self, pool_size: usize) -> Self {
        Self {
            pool_size: Some(pool_size),
            ..self
        }
    }
}

#[async_trait]
impl ResourceBuilder<Pool<AsyncPgConnection>> for Postgres {
    const TYPE: Type = Type::Database(DatabaseType::Shared(SharedEngine::Postgres));

    type Config = Self;
    type Output = (String, Option<usize>);

    fn new() -> Self {
        Self::default()
    }

    fn config(&self) -> &Self::Config {
        self
    }

    async fn output(
        self,
        factory: &mut dyn Factory,
    ) -> Result<Self::Output, shuttle_service::Error> {
        let conn_data = factory
            .get_db_connection(DatabaseType::Shared(SharedEngine::Postgres))
            .await?;
        let conn_string = conn_data.connection_string_private();

        if let Some(migrations) = self.migrations {
            let mut conn = AsyncPgConnection::establish(&conn_string)
                .await
                .map_err(|err| shuttle_service::Error::Custom(err.into()))?;

            conn.run_pending_migrations(migrations).await.unwrap();
        }

        Ok((conn_string, self.pool_size))
    }

    async fn build(
        (conn_string, pool_size): &Self::Output,
    ) -> Result<Pool<AsyncPgConnection>, shuttle_service::Error> {
        let config = AsyncDieselConnectionManager::new(conn_string);
        let mut pool = Pool::builder(config);
        if let Some(pool_size) = pool_size {
            pool = pool.max_size(*pool_size);
        }

        pool.build().map_err(|err| shuttle_service::Error::Custom(err.into()))
    }
}
