use mysql::prelude::Queryable;
use mysql::Conn;
use mysql::OptsBuilder;
use tracing::info;

use crate::util::exception::Exception;

pub struct MySQLClient {
    conn: Conn,
}

impl MySQLClient {
    pub fn new(host: &str, user: &str, password: &str) -> Result<Self, Exception> {
        let opts = OptsBuilder::new().ip_or_hostname(Some(host)).user(Some(user)).pass(Some(password));
        let conn = Conn::new(opts).map_err(Exception::from)?;
        Ok(Self { conn })
    }

    pub fn create_user(&mut self, user: &str, password: &str) -> Result<(), Exception> {
        info!("create user, user={}", user);
        let statement = format!("CREATE USER IF NOT EXISTS '{user}'@'%'");
        self.conn.exec_drop(statement, ()).map_err(Exception::from)?;

        let statement = format!("ALTER USER '{user}'@'%' IDENTIFIED BY '{password}'");
        self.conn.exec_drop(statement, ()).map_err(Exception::from)?;

        Ok(())
    }

    pub fn grant_user_privileges(&mut self, user: &str, dbs: &[&str], privileges: &[&str]) -> Result<(), Exception> {
        info!("grant user privileges, user={}, dbs={:?}, privileges={:?}", user, dbs, privileges);

        for db in dbs {
            let statement = format!("GRANT {} ON {}.* TO '{}'@'%'", privileges.join(", "), escape_db(db), user);
            self.conn.exec_drop(statement, ()).map_err(Exception::from)?;
        }

        Ok(())
    }
}

fn escape_db(db: &str) -> String {
    if db == "*" {
        return "*".to_string();
    }
    '`'.to_string() + db + "`"
}
