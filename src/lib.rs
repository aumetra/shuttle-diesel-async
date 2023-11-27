use async_trait::async_trait;
use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
    AsyncPgConnection,
};
use serde::Serialize;
use shuttle_service::{
    database::{SharedEngine, Type as DatabaseType},
    DbInput, DbOutput, Factory, ResourceBuilder, Type,
};

pub use diesel_async;

const MAX_POOL_SIZE: usize = 5;

#[derive(Default, Serialize)]
pub struct Postgres {
    #[serde(flatten)]
    db_input: DbInput,
}

impl Postgres {
    pub fn local_uri(self, local_uri: impl ToString) -> Self {
        Self {
            db_input: DbInput {
                local_uri: Some(local_uri.to_string()),
            },
        }
    }
}

fn get_connection_string(db_output: &DbOutput) -> String {
    match db_output {
        DbOutput::Info(ref info) => info.connection_string_private(),
        DbOutput::Local(ref local) => local.clone(),
    }
}

#[async_trait]
impl ResourceBuilder<Pool<AsyncPgConnection>> for Postgres {
    const TYPE: Type = Type::Database(DatabaseType::Shared(SharedEngine::Postgres));

    type Config = Self;
    type Output = DbOutput;

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
        let db_output = if let Some(local_uri) = self.db_input.local_uri {
            DbOutput::Local(local_uri)
        } else {
            let conn_data = factory
                .get_db_connection(DatabaseType::Shared(SharedEngine::Postgres))
                .await?;
            DbOutput::Info(conn_data)
        };

        Ok(db_output)
    }

    async fn build(
        db_output: &Self::Output,
    ) -> Result<Pool<AsyncPgConnection>, shuttle_service::Error> {
        let conn_string = get_connection_string(db_output);
        let config = AsyncDieselConnectionManager::new(conn_string);
        Pool::builder(config)
            .max_size(MAX_POOL_SIZE)
            .build()
            .map_err(|err| shuttle_service::Error::Custom(err.into()))
    }
}
