# shuttle-diesel-async

`diesel-async` resource for Shuttle services

## License

`shuttle-diesel-async` is licensed under the [MIT license](http://opensource.org/licenses/MIT).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, 
shall be licensed as above, without any additional terms or conditions.

## Usage

Add the crate, with [deadpool](https://docs.rs/deadpool/latest/deadpool/index.html) or [bb8](https://docs.rs/bb8/latest/bb8/). If you haven't already, also add your shuttle dependencies. 

This example demonstrates usage with postgres and deadpool.

```sh
cargo add --git "https://github.com/aumetra/shuttle-diesel-async" -F "deadpool"
# or
cargo add --git "https://github.com/aumetra/shuttle-diesel-async" -F "bb8"

## diesel dependencies
cargo add diesel -F postgres
cargo add diesel-async -F postgres -F deadpool

## other shuttle dependencies you may want
cargo add shuttle-runtime  # or cargo add shuttle-runtime --no-default-features
# to load secrets, like the DATABASE_URL from your environment
cargo add shuttle-secrets

# the following example demonstrates with shuttle-axum
cargo add axum 
cargo add shuttle-axum
```

Adjust your `main` function, (We will use `ShuttleAxum` as example) to set up your database connection, and add fields in your `Secrets.toml` accordingly.

Secrets.toml:

```toml
DB_PASSWORD="postgres" # or whatever you have it set to
DB_PORT="5432" # default port for postgres
```

```rust
use diesel::result::Error;
use diesel_async::{
  pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
  AsyncConnection, RunQueryDsl,
};

#[derive(Clone)]
pub struct SharedState {
  pub pool: Pool<AsyncPgConnection>,
}

#[shuttle_runtime::main]
async fn main(
  #[shuttle_secrets::Secrets] secret_store: shuttle_secrets::SecretStore,
  // shuttle will provision the database connection
  #[shuttle_shared_db::Postgres(
    local_uri = "postgres://postgres:{secrets.DB_PASSWORD}@localhost:{secrets.DB_PORT}/postgres"
  )]
  conn_str: String,
) -> shuttle_axum::ShuttleAxum {

  let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(conn_str);
  let pool = Pool::builder(config).build().unwrap();
  let shared_state = SharedState { pool };

  let router = Router::new()
    .route("/", get(handler))
    .with_state(shared_state);

  Ok(router.into())
}

use axum::extract::State;
async fn handler(State(state): State<SharedState>) {
  // get the connection from the pool
  let mut conn = state.pool.get().await.unwrap();
  // and you're good to go.
  crate::schema::your_table_name::dsl::your_table_name.select(YourTableStruct::as_select()).load(conn).await.unwrap()
}
```

Refer to [this example](https://github.com/tokio-rs/axum/blob/main/examples/diesel-async-postgres/src/main.rs) for further direction on using diesel, and the [shuttle docs](https://docs.shuttle.rs/resources/shuttle-shared-db#connection-string) on `shuttle-shared-db`.