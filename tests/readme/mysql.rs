use drizzle::mysql::{mysql_sync::Drizzle, prelude::*};

#[MySQLTable]
pub struct User {
    #[column(PRIMARY, AUTO_INCREMENT)]
    pub id: u64,
    #[column(VARCHAR(255))]
    pub name: String,
}

#[derive(MySQLSchema)]
pub struct Schema {
    pub users: User,
}

pub fn database() -> Result<(Drizzle<mysql::Conn, Schema>, Schema), mysql::Error> {
    let options = mysql::Opts::from_url("mysql://drizzle:drizzle@127.0.0.1:3307/drizzle_test")?;
    let connection = mysql::Conn::new(options)?;
    Ok(Drizzle::new(connection, Schema::new()))
}
