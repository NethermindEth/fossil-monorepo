use sqlx::Error;
use std::sync::Arc;

use crate::DbConnection;

#[derive(sqlx::FromRow, Debug)]
pub struct BlockHeader {
    pub block_hash: Option<String>,
    pub number: i64,
    pub gas_limit: Option<i64>,
    pub gas_used: Option<i64>,
    pub nonce: Option<String>,
    pub transaction_root: Option<String>,
    // base_fee_per_gas is going to be the main one we use here.
    pub base_fee_per_gas: Option<String>,

    pub receipts_root: Option<String>,
    pub state_root: Option<String>,
    pub timestamp: Option<i64>,
}

// This is the function to get all block headers information, useful for debugging
// However, it might be a lot faster if we only get the base_fee_per_gas information,
// which is what we will do in the production code.
pub async fn get_block_headers_by_time_range(
    db: Arc<DbConnection>,
    start_timestamp: i64,
    end_timestamp: i64,
) -> Result<Vec<BlockHeader>, Error> {
    tracing::debug!(
        "Getting block headers by time range: {} to {}",
        start_timestamp,
        end_timestamp
    );

    let headers = sqlx::query_as(
        r#"
        SELECT 
            block_hash, 
            number, 
            gas_limit, 
            gas_used, 
            base_fee_per_gas, 
            nonce, 
            transaction_root, 
            receipts_root, 
            state_root,
            timestamp
        FROM blockheaders
        WHERE CAST(timestamp AS BIGINT) BETWEEN $1 AND $2
        ORDER BY number ASC
        "#,
    )
    .bind(start_timestamp)
    .bind(end_timestamp)
    .fetch_all(&db.pool)
    .await?;

    Ok(headers)
}

pub async fn get_block_base_fee_by_time_range(
    db: Arc<DbConnection>,
    start_timestamp: i64,
    end_timestamp: i64,
) -> Result<Vec<String>, Error> {
    tracing::debug!(
        "Getting block headers by time range: {} to {}",
        start_timestamp,
        end_timestamp
    );

    let base_gas_fees = sqlx::query_scalar(
        r#"
        SELECT base_fee_per_gas
        FROM blockheaders
        WHERE CAST(timestamp AS BIGINT) BETWEEN $1 AND $2
        ORDER BY number ASC
        "#,
    )
    .bind(start_timestamp)
    .bind(end_timestamp)
    .fetch_all(&db.pool)
    .await?;

    Ok(base_gas_fees)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use testcontainers::{Container, clients::Cli, images::postgres::Postgres};

    lazy_static! {
        static ref DOCKER: Cli = Cli::default();
    }

    struct TestDb {
        db: Arc<DbConnection>,
        _container: Container<'static, Postgres>,
    }

    async fn setup_db() -> TestDb {
        let container = DOCKER.run(Postgres::default());
        let port = container.get_host_port_ipv4(5432);
        let connection_string = format!("postgres://postgres:postgres@localhost:{}/postgres", port);

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await
            .expect("Failed to create database pool");

        // Create a sample table with block headers for testing
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS blockheaders (
                block_hash TEXT,
                number BIGINT,
                gas_limit BIGINT,
                gas_used BIGINT,
                nonce TEXT,
                transaction_root TEXT,
                base_fee_per_gas TEXT,
                receipts_root TEXT,
                state_root TEXT,
                timestamp BIGINT
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create block_headers table");

        // Insert sample data
        sqlx::query(
            r#"
            INSERT INTO blockheaders 
            (block_hash, number, gas_limit, gas_used, nonce, transaction_root, base_fee_per_gas, receipts_root, state_root, timestamp)
            VALUES 
            ('0x1', 8006481, 100, 50, '0xnonce1', '0xtx1', '0xa0ba15', '0xreceipt1', '0xstate1', 1743249000),
            ('0x2', 8006482, 100, 50, '0xnonce2', '0xtx2', '0x9ed346', '0xreceipt2', '0xstate2', 1743249060),
            ('0x3', 8006483, 100, 50, '0xnonce3', '0xtx3', '0xa85f1d', '0xreceipt3', '0xstate3', 1743249090),
            ('0x4', 8006484, 100, 50, '0xnonce4', '0xtx4', '0x9aeae1', '0xreceipt4', '0xstate4', 1743249110),
            ('0x5', 8006485, 100, 50, '0xnonce5', '0xtx5', '0x9fda11', '0xreceipt5', '0xstate5', 1743249120)
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to insert sample data");

        let db = Arc::new(DbConnection { pool });

        TestDb {
            db,
            _container: container,
        }
    }

    #[tokio::test]
    async fn test_should_get_all_block_headers_by_time_range() {
        let test_db = setup_db().await;

        let headers = get_block_headers_by_time_range(test_db.db, 1743249000, 1743249120)
            .await
            .unwrap();

        assert_eq!(headers.len(), 5);
        assert_eq!(headers[0].number, 8006481);
        assert_eq!(headers[1].number, 8006482);
        assert_eq!(headers[2].number, 8006483);
        assert_eq!(headers[3].number, 8006484);
        assert_eq!(headers[4].number, 8006485);
    }

    #[tokio::test]
    async fn test_should_only_get_partial_block_headers_by_time_range() {
        let test_db = setup_db().await;

        let headers = get_block_headers_by_time_range(test_db.db, 1743249000, 1743249100)
            .await
            .unwrap();

        assert_eq!(headers.len(), 3);
        assert_eq!(headers[0].number, 8006481);
        assert_eq!(headers[1].number, 8006482);
        assert_eq!(headers[2].number, 8006483);
    }

    #[tokio::test]
    async fn test_should_get_block_headers_by_time_range_with_no_results() {
        let test_db = setup_db().await;

        // Use a time range that definitely won't match any results
        let headers = get_block_headers_by_time_range(test_db.db, 1643249000, 1643249100)
            .await
            .unwrap();

        assert_eq!(headers.len(), 0);
    }

    #[tokio::test]
    async fn test_should_get_all_block_base_fee_by_time_range() {
        let test_db = setup_db().await;

        let base_fees = get_block_base_fee_by_time_range(test_db.db, 1743249000, 1743249120)
            .await
            .unwrap();

        assert_eq!(base_fees.len(), 5);
        assert_eq!(base_fees[0], "0xa0ba15");
        assert_eq!(base_fees[1], "0x9ed346");
        assert_eq!(base_fees[2], "0xa85f1d");
        assert_eq!(base_fees[3], "0x9aeae1");
        assert_eq!(base_fees[4], "0x9fda11");
    }

    #[tokio::test]
    async fn test_should_only_get_partial_block_base_fee_by_time_range() {
        let test_db = setup_db().await;

        let base_fees = get_block_base_fee_by_time_range(test_db.db, 1743249000, 1743249100)
            .await
            .unwrap();

        assert_eq!(base_fees.len(), 3);
        assert_eq!(base_fees[0], "0xa0ba15");
        assert_eq!(base_fees[1], "0x9ed346");
        assert_eq!(base_fees[2], "0xa85f1d");
    }

    #[tokio::test]
    async fn test_should_get_block_base_fee_by_time_range_with_no_results() {
        let test_db = setup_db().await;

        // Use a time range that definitely won't match any results
        let base_fees = get_block_base_fee_by_time_range(test_db.db, 1643249000, 1643249100)
            .await
            .unwrap();

        assert_eq!(base_fees.len(), 0);
    }
}
