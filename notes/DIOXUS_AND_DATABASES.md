# [Working with Databases](https://dioxuslabs.com/learn/0.7/tutorial/databases#working-with-databases)

Our HotDog app is coming together nicely! We implemented a very simple backend that saves the user's favorite dog images to a local "dogs.txt" file.

In practice, you will likely want to store data in a proper database. Modern databases are _much_ more powerful than a text file!

If you already have a good understanding of databases, jump ahead to the [section where we integrate Sqlite with HotDog](https://dioxuslabs.com/learn/0.7/tutorial/databases#adding-database-operations-to-hotdog).

## [Choosing a Database](https://dioxuslabs.com/learn/0.7/tutorial/databases#choosing-a-database)

In today's age of app development, there are _many_ databases to choose from, each with their own strengths, weaknesses, and tradeoffs to consider. For apps with just a few users, it is fine to select a "simpler" database that's easier to manage. For apps with many users, you might want to consider more advanced databases with additional tooling to meet stricter requirements.

Here is a (incomplete!) list of databases and a short summary of each:

- [PostgreSQL](https://www.postgresql.org/): Advanced database known for its powerful plugin system.
- [MySQL](https://www.mysql.com/): World's most popular open source database good for all apps.
- [SQLite](https://www.sqlite.org/): Simple file-based engine known for its reliability and embeddability.
- [Oracle](https://www.oracle.com/database/): Advanced commercial database known for its enterprise features.
- [Redis](http://redis.io/): Simple key-value database known for its great performance.
- [MongoDB](https://www.mongodb.com/): A database ideal for data that doesn't fit into rows and columns.
- [SurrealDB](https://surrealdb.com/): A new "all-in-one" database that combines many models.
- [CockroachDB](https://www.cockroachlabs.com/): Distributed SQL database designed for high-availability.
- [and many more](https://dev.to/shreyvijayvargiya/list-of-45-databases-in-the-world-57e8)!

There are many different types of databases, each good at different tasks. These might include:

- **Relational**: traditional row/column/table approach.
- **Document**: storing unstructured or loosely structured blobs of data.
- **Timeseries**: storing and querying lots of data that changes over time.
- **Graph**: querying data based on its connections to other data.
- **Key-value**: storing just key-value pairs - a fast concurrent HashMap.
- **In-memory**: designed for low-latency operations usually used as a cache.
- **Embedded**: a database that is shipped _inside_ your app.

For most apps - unless you have specific requirements - we recommend a mainstream relational database like PostgreSQL or MySQL.

> 📣 PostgreSQL is currently a very interesting option: it can be extended to support time-series, vector, graph, search and geo-spatial data with plugins.

In some cases, you might want a database that's specific to _just one app instance_ or the _user's machine_. In these cases, you'll want to use an embedded database like [SQLite](https://www.sqlite.org/) or [RocksDB](https://rocksdb.org/).

## [Adding Database operations to HotDog](https://dioxuslabs.com/learn/0.7/tutorial/databases#adding-database-operations-to-hotdog)

For _HotDog_, we're going to use Sqlite. _HotDog_ is a very simple app and will only ever have one user: you!

To add sqlite functionality to _HotDog_, we'll pull in the `rusqlite` crate. Note that `rusqlite` is only meant to be compiled on the server, so we'll feature gate it behind the `"server"` feature in our Cargo.toml.

```
[dependencies]
# ....
rusqlite = { version = "0.32.1", optional = true } # <--- add rusqlite
[features]
# ....
server = ["dioxus/server", "dep:rusqlite"] # <---- add dep:rusqlite
```

To connect to our database, we're going to use the `rusqlite::Connection`. Rusqlite connections are not thread-safe and must exist once-per-thread, so we'll need to wrap it in a thread_local.

When the connection is initialized, we'll run a SQL action to create the "dogs" table with our data.

```rust|src/guide_databases.rs
// The database is only available to server code
#[cfg(feature = "server")]
thread_local! {
    pub static DB: rusqlite::Connection = {
        // Open the database from the persisted "hotdog.db" file
        let conn = rusqlite::Connection::open("hotdog.db").expect("Failed to open database");
        // Create the "dogs" table if it doesn't already exist
        conn.execute_batch(            "CREATE TABLE IF NOT EXISTS dogs (
                id INTEGER PRIMARY KEY,                url TEXT NOT NULL            );",
        ).unwrap();
        // Return the connection
        conn    };
}
```

Now, in our `save_dog` server function, we can use SQL to insert the value into the database:

```rust|src/guide_databases.rs
#[server]
async fn save_dog(image: String) -> Result<()> {
    DB.with(|f| f.execute("INSERT INTO dogs (url) VALUES (?1)", &[&image]))?;
    Ok(())
}
```

Once the app is launched, you should see a "hotdog.db" file in your crate's directory. Let's save a few dog photos and then open the database in a database viewer. If all goes well, you should see the saved dog photos!

## [Notes on Databases and Rust](https://dioxuslabs.com/learn/0.7/tutorial/databases#notes-on-databases-and-rust)

While there are many database providers, Rust support can be limited. Rust is still a new choice for web development. In this section we'll provide our own (biased!) opinions on what libraries we recommend for interacting with databases.

It's also important to note that several libraries exist at a higher level abstraction than raw SQL. These are called an _Object Relationship Mapper (ORM)_. Rust ORM libraries map the SQL language into ordinary Rust functions. We generally recommend just sticking with SQL, but ORMs can make working writing some queries easier.

- [Sqlx](https://github.com/launchbadge/sqlx): A straightforward yet large interface to Postgres, MySql, and Sqlite.
- [SeaORM](https://github.com/SeaQL/sea-orm): An ORM built on top of Sqlx for deriving databases.
- [rusqlite](https://github.com/rusqlite/rusqlite): An intuitive sqlite interface with no special ORM magic.
- [rust-postgres](https://github.com/sfackler/rust-postgres): An interface to Postgres with an API similar to rusqlite.
- [Turbosql](https://github.com/trevyn/turbosql): A _very_ terse interface to Sqlite with automatic derives.

We aren't including libraries like [Diesel](http://diesel.rs/) in this list since it seems that the Rust ecosystem has evolved towards newer projects with 1st-class async support.

There are many libraries we haven't tested yet, but might be worth checking out:

- [firebase-rs](https://github.com/emreyalvac/firebase-rs): Firebase client crate
- [postgrest-rs](https://github.com/supabase-community/postgrest-rs): Supabase client crate
- [mongo-rust-driver](https://github.com/mongodb/mongo-rust-driver): Official MongoDB client crate

## [Choosing a Database Provider](https://dioxuslabs.com/learn/0.7/tutorial/databases#choosing-a-database-provider)

While there are just a handful of databases you might consider for your app, there are many _database providers_, each with their own strengths and weaknesses. We are not sponsored by any of these providers - this is just a list of providers we have seen in use by Rust apps.

You _do not_ need to use a database provider. Databases providers provide paid database hosting. It will cost you money to use these providers! Many have a free tier and some support "scale-to-zero" to help you save money on small apps. At any time, you are free to host and manage your own database.

For popular relational databases:

- [GCP](https://cloud.google.com/products/databases): Provides AlloyDB (enterprise postgres), CloudSQL (MySql, Postgres), and more.
- [AWS](https://aws.amazon.com/products/databases/): Provides RDS, Aurora, DynamoDB, and more.
- [PlanetScale](https://planetscale.com/): Reliable MySQL-compatible database with sharding designed for scale.
- [Firebase](https://firebase.google.com/): Google's comprehensive real-time database designed for rapid app development.
- [Supabase](https://supabase.com/): Hosted Postgres known for its great dashboard and tooling.
- [Neon](https://neon.tech/): Hosted Postgres that separates compute and storage for scale-to-zero apps.

For Sqlite:

- [LiteFS](https://fly.io/docs/litefs/): A distributed Sqlite sync engine designed to be used with Fly.io
- [Turso](https://turso.tech/): A "multi-tenant" sqlite provider that maintains one isolated database per user

The "scale-to-zero" relational solutions:

- [AWS Aurora](https://aws.amazon.com/rds/aurora/)
- [LiteFS](https://fly.io/docs/litefs/)

We don't suggest any particular database provider.

- If you have lots of free cloud credits, consider AWS/GCP/Azure.
- If you want Postgres with a good dashboard, consider Supabase or Neon.
- If you want a simple experience, consider Turso or LiteFS.
