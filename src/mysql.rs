use log::info;
use mysql::Conn;
use mysql::OptsBuilder;
use mysql::SslOpts;
use mysql::prelude::Queryable;

pub struct MySQLClient {
    conn: Conn,
}

impl MySQLClient {
    pub fn new(host: &str, user: &str, password: &str) -> Self {
        let opts = OptsBuilder::new()
            .ip_or_hostname(Some(host))
            .user(Some(user))
            .pass(Some(password))
            .ssl_opts(SslOpts::default().with_danger_accept_invalid_certs(true));

        let conn = Conn::new(opts).unwrap_or_else(|err| panic!("{err}"));
        Self { conn }
    }

    pub fn create_db(&mut self, db: &str) {
        info!("create db, db={db}");
        let statement = format!("CREATE DATABASE IF NOT EXISTS `{db}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci");
        self.conn.exec_drop(statement, ()).unwrap_or_else(|err| panic!("{err}"));
    }

    pub fn create_user(&mut self, user: &str, password: &str) {
        info!("create user, user={}", user);
        let statement = format!("CREATE USER IF NOT EXISTS '{user}'@'%'");
        self.conn.exec_drop(statement, ()).unwrap_or_else(|err| panic!("{err}"));

        let statement = format!("ALTER USER '{user}'@'%' IDENTIFIED BY '{password}'");
        self.conn.exec_drop(statement, ()).unwrap_or_else(|err| panic!("{err}"));
    }

    pub fn grant_user_privileges(&mut self, user: &str, dbs: &[&str], privileges: &[&str]) {
        info!("grant user privileges, user={}, dbs={:?}, privileges={:?}", user, dbs, privileges);

        for db in dbs {
            let statement = format!("GRANT {} ON {}.* TO '{}'@'%'", privileges.join(", "), escape_db(db), user);
            self.conn.exec_drop(statement, ()).unwrap_or_else(|err| panic!("{err}"));
        }
    }
}

fn escape_db(db: &str) -> String {
    if db == "*" {
        return "*".to_string();
    }
    format!("`{db}`")
}
