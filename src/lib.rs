#![deny(clippy::all)]

mod absolute;
mod combined;
mod errors;
mod relative;

pub use absolute::AbsolutePath;
pub use absolute::AbsolutePathBuf;
pub use combined::CombinedPath;
pub use combined::CombinedPathBuf;
pub use errors::*;
pub use relative::RelativePath;
pub use relative::RelativePathBuf;

#[cfg(all(test, feature = "diesel"))]
#[macro_use]
extern crate diesel;

// Absolute + Relative
// Serialize / Deserialize
// Buf / NonBuf versions
// Display
// db serializable
// Normalize
// Check "join" is relative for RelativePath

#[cfg(all(test, feature = "diesel"))]
pub(crate) mod diesel_helpers {
    use diesel::sql_query;
    use diesel::Connection;
    use diesel::RunQueryDsl;
    use diesel::SqliteConnection;

    pub(crate) mod schema {
        table! {
            test_files (id) {
                id -> Integer,
                x -> Text,
                y -> Nullable<Text>,
            }
        }
    }

    pub(crate) fn create_table() -> anyhow::Result<SqliteConnection> {
        let mut connection = diesel::sqlite::SqliteConnection::establish(":memory:")?;
        diesel::sql_query(
            "CREATE TABLE test_files (id PRIMARY KEY NOT NULL, x TEXT NOT NULL, y TEXT NULL)",
        )
        .execute(&mut connection)?;
        Ok(connection)
    }

    pub(crate) fn insert_values(
        connection: &mut SqliteConnection,
        values: &[(i32, &str, Option<&str>)],
    ) -> anyhow::Result<()> {
        for (pk, x, y) in values {
            let y = match y {
                None => "NULL".to_owned(),
                Some(value) => {
                    format!("\"{}\"", value)
                }
            };
            sql_query(format!(
                "INSERT INTO test_files (id, x, y) VALUES ({}, \"{}\", {});",
                pk, x, y
            ))
            .execute(connection)?;
        }
        Ok(())
    }
}
