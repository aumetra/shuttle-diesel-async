use async_trait::async_trait;
use diesel_async::{AsyncConnection, AsyncPgConnection};
use serde::{Deserialize, Serialize};
use shuttle_service::{
    database::{SharedEngine, Type as DatabaseType},
    resource::Type,
    DatabaseResource, DbInput, Factory, IntoResource, ResourceBuilder,
};

#[cfg(any(feature = "bb8", feature = "deadpool"))]
use diesel_async::pooled_connection::AsyncDieselConnectionManager;

#[cfg(feature = "bb8")]
use diesel_async::pooled_connection::bb8;

#[cfg(feature = "deadpool")]
use diesel_async::pooled_connection::deadpool;

pub use diesel_async;

#[cfg(any(feature = "bb8", feature = "deadpool"))]
const MAX_POOL_SIZE: usize = 5;

#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct Wrapper(DatabaseResource);

#[derive(Default)]
pub struct Postgres {
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

#[inline]
#[cfg(any(feature = "bb8", feature = "deadpool"))]
fn get_pool_manager(
    db_output: &DatabaseResource,
) -> AsyncDieselConnectionManager<AsyncPgConnection> {
    AsyncDieselConnectionManager::new(get_connection_string(db_output))
}

#[inline]
fn get_connection_string(db_output: &DatabaseResource) -> String {
    match db_output {
        DatabaseResource::ConnectionString(conn_str) => conn_str.clone(),
        DatabaseResource::Info(info) => info.connection_string_shuttle(),
    }
}

#[async_trait]
impl ResourceBuilder for Postgres {
    const TYPE: Type = Type::Database(DatabaseType::Shared(SharedEngine::Postgres));

    type Config = DbInput;
    type Output = Wrapper;

    fn config(&self) -> &Self::Config {
        &self.db_input
    }

    async fn output(
        self,
        factory: &mut dyn Factory,
    ) -> Result<Self::Output, shuttle_service::Error> {
        let resource = if let Some(local_uri) = self.db_input.local_uri {
            DatabaseResource::ConnectionString(local_uri)
        } else {
            let conn_info = factory
                .get_db_connection(DatabaseType::Shared(SharedEngine::Postgres))
                .await?;
            DatabaseResource::Info(conn_info)
        };

        Ok(Wrapper(resource))
    }
}

#[async_trait]
impl IntoResource<AsyncPgConnection> for Wrapper {
    async fn into_resource(self) -> Result<AsyncPgConnection, shuttle_service::Error> {
        AsyncPgConnection::establish(&get_connection_string(&self.0))
            .await
            .map_err(|err| shuttle_service::Error::Database(err.to_string()))
    }
}

#[async_trait]
#[cfg(feature = "bb8")]
impl IntoResource<bb8::Pool<AsyncPgConnection>> for Wrapper {
    async fn into_resource(self) -> Result<bb8::Pool<AsyncPgConnection>, shuttle_service::Error> {
        bb8::Pool::builder()
            .max_size(MAX_POOL_SIZE as u32)
            .build(get_pool_manager(&self.0))
            .await
            .map_err(|err| shuttle_service::Error::Database(err.to_string()))
    }
}

#[async_trait]
#[cfg(feature = "deadpool")]
impl IntoResource<deadpool::Pool<AsyncPgConnection>> for Wrapper {
    async fn into_resource(
        self,
    ) -> Result<deadpool::Pool<AsyncPgConnection>, shuttle_service::Error> {
        deadpool::Pool::builder(get_pool_manager(&self.0))
            .max_size(MAX_POOL_SIZE)
            .build()
            .map_err(|err| shuttle_service::Error::Database(err.to_string()))
    }
}
