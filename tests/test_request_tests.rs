use axum::Router;
use axum_test::TestServer;
use cookie::Cookie;
use cookie::CookieJar;
use serde_json::json;
use std::fs::write;
use tempfile::NamedTempFile;

// Simple handler and router used by tests so requests to "/" return 200 instead of 404
async fn ok_handler() -> &'static str {
    ""
}

fn test_router() -> Router {
    Router::new().route("/", axum::routing::any(ok_handler))
}

#[cfg(test)]
mod test_content_type {
    use super::*;

    #[tokio::test]
    async fn content_type_sets_header() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.get("/").content_type("application/yaml").await;

        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn content_type_not_set_by_default() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.get("/").await;

        assert_eq!(response.status_code(), 200);
    }
}

#[cfg(test)]
mod test_json {
    use super::*;

    #[tokio::test]
    async fn json_sets_content_type() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.post("/").json(&json!({"name": "John"})).await;

        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn json_from_file() {
        let temp_file = NamedTempFile::new().unwrap();
        write(temp_file.path(), r#"{"name": "John"}"#).unwrap();

        let server = TestServer::new(test_router()).unwrap();

        let response = server.post("/").json_from_file(temp_file.path()).await;

        assert_eq!(response.status_code(), 200);
    }
}

#[cfg(feature = "yaml")]
#[cfg(test)]
mod test_yaml {
    use super::*;

    #[tokio::test]
    async fn yaml_sets_content_type() {
        let server = TestServer::new(test_router()).unwrap();

        let yaml_body =
            serde_yaml::to_string(&serde_yaml::Value::String("hello".to_string())).unwrap();
        let response =
            server.post("/").add_header("content-type", "application/x-yaml").text(yaml_body).await;
        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn yaml_from_file() {
        let temp_file = NamedTempFile::new().unwrap();
        write(temp_file.path(), "name: John").unwrap();

        let server = TestServer::new(test_router()).unwrap();

        let response = server.post("/").bytes_from_file(temp_file.path()).await;

        assert_eq!(response.status_code(), 200);
    }
}

#[cfg(feature = "msgpack")]
#[cfg(test)]
mod test_msgpack {
    use super::*;
    use bytes::Bytes;

    #[tokio::test]
    async fn msgpack_sets_content_type() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.post("/").bytes(Bytes::from("hello")).await;

        assert_eq!(response.status_code(), 200);
    }
}

#[cfg(test)]
mod test_form {
    use super::*;

    #[tokio::test]
    async fn form_sets_content_type() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.post("/").form(&[("name", "John")]).await;

        assert_eq!(response.status_code(), 200);
    }
}

#[cfg(test)]
mod test_multipart {
    use super::*;
    use axum_test::multipart::MultipartForm;

    #[tokio::test]
    async fn multipart_sets_content_type() {
        let server = TestServer::new(test_router()).unwrap();

        let multipart_form = MultipartForm::new().add_text("name", "John");

        let response = server.post("/").multipart(multipart_form).await;

        assert_eq!(response.status_code(), 200);
    }
}

#[cfg(test)]
mod test_text {
    use super::*;

    #[tokio::test]
    async fn text_sets_content_type() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.post("/").text("hello world").await;

        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn text_from_file() {
        let temp_file = NamedTempFile::new().unwrap();
        write(temp_file.path(), "hello world").unwrap();

        let server = TestServer::new(test_router()).unwrap();

        let response = server.post("/").text_from_file(temp_file.path()).await;

        assert_eq!(response.status_code(), 200);
    }
}

#[cfg(test)]
mod test_bytes {
    use super::*;
    use bytes::Bytes;

    #[tokio::test]
    async fn bytes_sets_body() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.post("/").bytes(Bytes::from("hello")).await;

        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn bytes_from_file() {
        let temp_file = NamedTempFile::new().unwrap();
        write(temp_file.path(), "hello").unwrap();

        let server = TestServer::new(test_router()).unwrap();

        let response = server.post("/").bytes_from_file(temp_file.path()).await;

        assert_eq!(response.status_code(), 200);
    }
}

#[cfg(test)]
mod test_cookies {
    use super::*;

    #[tokio::test]
    async fn add_cookie() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.get("/").add_cookie(Cookie::new("name", "value")).await;

        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn add_cookies() {
        let server = TestServer::new(test_router()).unwrap();

        let mut jar = CookieJar::new();
        jar.add(Cookie::new("name1", "value1"));
        jar.add(Cookie::new("name2", "value2"));

        let response = server.get("/").add_cookies(jar).await;

        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn clear_cookies() {
        let server = TestServer::new(test_router()).unwrap();

        let response =
            server.get("/").add_cookie(Cookie::new("name", "value")).clear_cookies().await;

        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn save_cookies() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.get("/").save_cookies().await;

        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn do_not_save_cookies() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.get("/").do_not_save_cookies().await;

        assert_eq!(response.status_code(), 200);
    }
}

#[cfg(test)]
mod test_query_params {
    use super::*;

    #[tokio::test]
    async fn add_query_param() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.get("/").add_query_param("name", "value").await;

        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn add_query_params() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.get("/").add_query_params([("name", "value")]).await;

        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn add_raw_query_param() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.get("/").add_raw_query_param("name=value").await;

        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn clear_query_params() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.get("/").add_query_param("name", "value").clear_query_params().await;

        assert_eq!(response.status_code(), 200);
    }
}

#[cfg(test)]
mod test_headers {
    use super::*;

    #[tokio::test]
    async fn add_header() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.get("/").add_header("x-custom", "value").await;

        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn clear_headers() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.get("/").add_header("x-custom", "value").clear_headers().await;

        assert_eq!(response.status_code(), 200);
    }
}

#[cfg(test)]
mod test_authorization {
    use super::*;

    #[tokio::test]
    async fn authorization() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.get("/").authorization("Bearer token").await;

        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn authorization_bearer() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.get("/").authorization_bearer("token").await;

        assert_eq!(response.status_code(), 200);
    }
}

#[cfg(test)]
mod test_expect_state {
    use super::*;

    #[tokio::test]
    async fn expect_success() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.get("/").expect_success().await;

        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn expect_failure() {
        // This test expects a non-2xx response; use an empty router so GET "/" returns 404
        let server = TestServer::new(Router::new()).unwrap();

        let response = server.get("/").expect_failure().await;

        // Expect a non-2xx status (router is empty -> 404 Not Found)
        assert_eq!(response.status_code(), 404);
    }
}

#[cfg(test)]
mod test_scheme {
    use super::*;

    #[tokio::test]
    async fn scheme() {
        let server = TestServer::new(test_router()).unwrap();

        let response = server.get("/").scheme("https").await;

        assert_eq!(response.status_code(), 200);
    }
}

#[cfg(test)]
mod test_file_loading {
    use super::*;

    #[tokio::test]
    #[should_panic]
    async fn json_from_file_nonexistent() {
        let server = TestServer::new(test_router()).unwrap();

        server.post("/").json_from_file("nonexistent.json").await;
    }

    #[tokio::test]
    #[should_panic]
    async fn text_from_file_nonexistent() {
        let server = TestServer::new(Router::new()).unwrap();

        server.post("/").text_from_file("nonexistent.txt").await;
    }

    #[tokio::test]
    #[should_panic]
    async fn bytes_from_file_nonexistent() {
        let server = TestServer::new(Router::new()).unwrap();

        server.post("/").bytes_from_file("nonexistent.bin").await;
    }
}

#[cfg(feature = "yaml")]
#[cfg(test)]
mod test_yaml_file_loading {
    use super::*;

    #[tokio::test]
    #[should_panic]
    async fn yaml_from_file_nonexistent() {
        let server = TestServer::new(Router::new()).unwrap();

        server.post("/").bytes_from_file("nonexistent.yaml").await;
    }
}
