use std::net::TcpStream;
use std::io::{Read, Write};
use crate::db::DB_URL;
use postgres::{Client, NoTls};
use crate::models::User;
use crate::utils::{get_id, get_user_request_body};

// HTTP 응답 상태 상수
const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const INTERNAL_SERVER_ERROR: &str = "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";

// 클라이언트 요청 처리 함수
pub fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer) {
        Ok(size) => {
            request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());

            let (status_line, content) = match &*request {
                r if r.starts_with("POST /users") => {
                    println!("POST /users 요청 도착");
                    handle_post_request(r)
                },
                r if r.starts_with("GET /users/") => {
                    println!("GET /users/ 요청 도착");
                    handle_get_request(r)
                },
                r if r.starts_with("GET /users") => {
                    println!("GET /users 요청 도착");
                    handle_get_all_request(r)
                },
                r if r.starts_with("PUT /users/") => {
                    println!("PUT /users/ 요청 도착");
                    handle_put_request(r)
                },
                r if r.starts_with("DELETE /users/") => {
                    println!("DELETE /users/ 요청 도착");
                    handle_delete_request(r)
                },
                r if r.starts_with("GET /health") => {
                    println!("GET /health 요청 도착");
                    handle_health_check(r)
                },
                _ => (NOT_FOUND.to_string(), "404 Not Found".to_string()),
            };

            stream.write_all(format!("{}{}", status_line, content).as_bytes()).unwrap();
        }
        Err(e) => {
            println!("스트림 읽기 중 오류 발생: {}", e);
        }
    }
}

// 헬스 체크 API
fn handle_health_check(_request: &str) -> (String, String) {
    (OK_RESPONSE.to_string(), "서버가 정상적으로 작동 중입니다.".to_string())
}

// POST 요청 처리 함수
fn handle_post_request(request: &str) -> (String, String) {
    match (get_user_request_body(request), Client::connect(DB_URL, NoTls)) {
        (Ok(user), Ok(mut client)) => {
            client
                .execute(
                    "INSERT INTO users (name, email) VALUES ($1, $2)",
                    &[&user.name, &user.email]
                )
                .unwrap();

            (OK_RESPONSE.to_string(), "User created".to_string())
        }
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

// GET 요청 처리 함수 (특정 사용자)
fn handle_get_request(request: &str) -> (String, String) {
    match (get_id(request).parse::<i32>(), Client::connect(DB_URL, NoTls)) {
        (Ok(id), Ok(mut client)) =>
            match client.query_one("SELECT * FROM users WHERE id = $1", &[&id]) {
                Ok(row) => {
                    let user = User {
                        id: row.get(0),
                        name: row.get(1),
                        email: row.get(2),
                    };

                    (OK_RESPONSE.to_string(), serde_json::to_string(&user).unwrap())
                }
                _ => (NOT_FOUND.to_string(), "User not found".to_string()),
            }

        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

// GET 요청 처리 함수 (모든 사용자)
fn handle_get_all_request(_request: &str) -> (String, String) {
    match Client::connect(DB_URL, NoTls) {
        Ok(mut client) => {
            let mut users = Vec::new();

            for row in client.query("SELECT * FROM users", &[]).unwrap() {
                users.push(User {
                    id: row.get(0),
                    name: row.get(1),
                    email: row.get(2),
                });
            }

            (OK_RESPONSE.to_string(), serde_json::to_string(&users).unwrap())
        }
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

// PUT 요청 처리 함수
fn handle_put_request(request: &str) -> (String, String) {
    match
        (
            get_id(request).parse::<i32>(),
            get_user_request_body(request),
            Client::connect(DB_URL, NoTls),
        )
    {
        (Ok(id), Ok(user), Ok(mut client)) => {
            client
                .execute(
                    "UPDATE users SET name = $1, email = $2 WHERE id = $3",
                    &[&user.name, &user.email, &id]
                )
                .unwrap();

            (OK_RESPONSE.to_string(), "User updated".to_string())
        }
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

// DELETE 요청 처리 함수
fn handle_delete_request(request: &str) -> (String, String) {
    match (get_id(request).parse::<i32>(), Client::connect(DB_URL, NoTls)) {
        (Ok(id), Ok(mut client)) => {
            let rows_affected = client.execute("DELETE FROM users WHERE id = $1", &[&id]).unwrap();

            if rows_affected == 0 {
                return (NOT_FOUND.to_string(), "User not found".to_string());
            }

            (OK_RESPONSE.to_string(), "User deleted".to_string())
        }
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}
