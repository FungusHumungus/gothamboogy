use crate::redis::RedisConnection;
use argon2rs::argon2i_simple;
use rand::prelude::*;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    password: [u8; 32],
    salt: String,
}

impl User {
    /// Create a new user. The password is salted and hashed using argon2i.
    pub fn new(username: &str, password: &str) -> Self {
        let mut data = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut data);
        let salt = hex::encode(&data);

        User {
            username: String::from(username),
            password: argon2i_simple(&password, &salt),
            salt: salt.to_string(),
        }
    }
}

pub fn add_user(conn: &RedisConnection, user: User) {
    let ser = serde_json::to_string(&user).unwrap();
    conn.conn
        .send_and_forget(resp_array!["HSET", "user", user.username, ser]);
}

pub async fn validate_user(conn: &RedisConnection, username: &str, password: &str) -> Option<User> {
    conn.conn
        .send(resp_array!["HGET", "user", username])
        .await
        .ok()
        .and_then(|u: String| {
            serde_json::from_str(&u).ok().and_then(|user: User| {
                let hashed = argon2i_simple(&password, &user.salt);
                if hashed == user.password {
                    Some(user)
                } else {
                    None
                }
            })
        })
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_validates() {
        let mut users = Vec::new();
        let user = User::new("user", "amazingpassword");
        add_user(&mut users, user);

        assert_eq!(
            Some("user".to_string()),
            validate_user(&users, "user", "amazingpassword").map(|u| u.username.to_string())
        );
    }

    #[test]
    fn test_incorrect_password() {
        let mut users = Vec::new();
        let user = User::new("user", "amazingpassword");
        add_user(&mut users, user);

        assert_eq!(None, validate_user(&users, "user", "terrible_password"));
    }
}
*/
