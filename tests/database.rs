#![allow(unused_imports)]
#![allow(unused_parens)]
use log::trace;
use pretty_assertions::assert_eq;

use arangors::Connection;
use common::{
    connection, get_arangodb_host, get_normal_password, get_normal_user, get_root_password,
    get_root_user, test_root_and_normal, test_setup, collection
};

pub mod common;

const NEW_DB_NAME: &str = "example";

#[maybe_async::test(
    any(feature = "reqwest_blocking"),
    async(any(feature = "reqwest_async"), tokio::test),
    async(any(feature = "surf_async"), async_std::test)
)]
async fn test_create_and_drop_database() {
    test_setup();
    let host = get_arangodb_host();
    let root_user = get_root_user();
    let root_password = get_root_password();

    let conn = Connection::establish_jwt(&host, &root_user, &root_password)
        .await
        .unwrap();

    let result = conn.create_database(NEW_DB_NAME).await;
    if let Err(e) = result {
        assert!(false, "Fail to create database: {:?}", e)
    };
    let result = conn.db(NEW_DB_NAME).await;
    assert_eq!(result.is_err(), false);

    let result = conn.drop_database(NEW_DB_NAME).await;
    if let Err(e) = result {
        assert!(false, "Fail to drop database: {:?}", e)
    };
    let result = conn.db(NEW_DB_NAME).await;
    assert_eq!(result.is_err(), true);
}

#[maybe_async::test(
    any(feature = "reqwest_blocking"),
    async(any(feature = "reqwest_async"), tokio::test),
    async(any(feature = "surf_async"), async_std::test)
)]
async fn test_fetch_current_database_info() {
    test_setup();

    #[maybe_async::maybe_async]
    async fn fetch_current_database(user: String, passwd: String) {
        let host = get_arangodb_host();
        let conn = Connection::establish_jwt(&host, &user, &passwd)
            .await
            .unwrap();
        let db = conn.db("test_db").await.unwrap();
        let info = db.info().await;
        match info {
            Ok(info) => {
                trace!("{:?}", info);
                assert_eq!(info.is_system, false)
            }
            Err(e) => assert!(false, "Fail to fetch database: {:?}", e),
        }
    }
    test_root_and_normal(fetch_current_database).await;
}

#[maybe_async::test(
    any(feature = "reqwest_blocking"),
    async(any(feature = "reqwest_async"), tokio::test),
    async(any(feature = "surf_async"), async_std::test)
)]
async fn test_get_version() {
    test_setup();
    let conn = connection().await;
    let db = conn.db("test_db").await.unwrap();
    let version = db.arango_version().await.unwrap();
    trace!("{:?}", version);
    assert_eq!(version.license, "community");
    assert_eq!(version.server, "arango");

    let re = regex::Regex::new(r"3\.\d+\.\d+").unwrap();
    assert_eq!(
        re.is_match(&version.version),
        true,
        "version: {}",
        version.version
    );
}

#[maybe_async::test(
    any(feature = "reqwest_blocking"),
    async(any(feature = "reqwest_async"), tokio::test),
    async(any(feature = "surf_async"), async_std::test)
)]
async fn test_aql_query_job() {
    use arangors::AqlQuery;
    use serde::Deserialize;
    use serde_json::Value;

    test_setup();

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestDoc {
        _key: String,
        value: i32,
    }
    let collection_name = "test_collection_get_docs";
    let conn = connection().await;
    let database = conn.db("test_db").await.unwrap();
    let _ = collection(&conn, collection_name).await;
    // Insert test documents
    database
        .aql_str::<Value>(r#"
        INSERT { "_key": "doc1", "value": 1 } INTO test_collection_get_docs
        "#)
        .await
        .unwrap();

    let aql = AqlQuery::builder().query("FOR i IN test_collection_get_docs RETURN i").build();
    let job_id = database.aql_query_job::<Vec<TestDoc>>(aql).await.unwrap();
    println!("job_id: {}", job_id);
    let result = database.get_job_result::<Vec<TestDoc>>(&job_id).await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], TestDoc { _key: "doc1".to_string(), value: 1 });
}
