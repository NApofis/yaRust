
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::str::FromStr;

// use proc_my_macros::say_hello;
use proc_my_macros::FromSql;
use proc_my_macros::ToSql;

#[derive(Debug)]
enum Status {
    Online,
    Offline,
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Status::Online => write!(f, "Online"),
            Status::Offline => write!(f, "Offline"),
        }
    }
}

impl FromStr for Status {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Online" => Ok(Status::Online),
            "Offline" => Ok(Status::Offline),
            _ => Err(format!("Unknown status: {}", s)),
        }
    }
}

#[derive(Debug, ToSql, FromSql)]
struct User {
    id: i32,
    name: String,
    age: i32,
    // status: Status,
}
fn main() {

    // Старый код запущен

    let user = User {
        id: 1,
        name: "Alice".into(),
        age: 30,
        // status: Status::Online,
    };
    println!("{}", user.to_sql("users"));

    let sql = "INSERT INTO users (id, name,age, status) VALUES('1','Bob','35', 'Offline');";
    let user2 = User::from_sql(sql);
    println!("{:?}", user2);
}