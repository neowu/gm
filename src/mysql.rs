use std::error::Error;

use mysql::{params, prelude::Queryable, Conn, OptsBuilder};

pub struct MySQLClient {
    conn: Conn,
}

impl MySQLClient {
    pub fn new(host: &str, user: &str, password: &str) -> Result<MySQLClient, Box<dyn Error>> {
        let opts = OptsBuilder::new().ip_or_hostname(Some(host)).user(Some(user)).pass(Some(password));
        let conn = Conn::new(opts)?;
        Ok(MySQLClient { conn })
    }

    pub fn create_user(&mut self, user: &str, password: &str) -> Result<(), Box<dyn Error>> {
        println!("create user, user={}", user);
        let statement = format!("CREATE USER IF NOT EXISTS '{user}'@'%'");
        self.conn.exec_drop(statement, ())?;

        let statement = format!("ALTER USER '{user}'@'%' IDENTIFIED BY '{password}'");
        self.conn.exec_drop(statement, ())?;

        Ok(())
    }

    pub fn grant_user_privileges(&mut self, user: &str, dbs: &Vec<String>, privileges: &Vec<&str>) -> Result<(), Box<dyn Error>> {
        println!("grant user privileges, user={}, dbs={:?}, privileges={:?}", user, dbs, privileges);

        for db in dbs {
            let statement = format!("GRANT {} ON {}.* TO '{}'@'%'", privileges.join(", "), escape_db(db), user);
            self.conn.exec_drop(statement, ())?;
        }

        Ok(())
    }
}

fn escape_db(db: &str) -> String {
    if db == "*" {
        return "*".to_string();
    }
    format!("`{}`", db)
}
