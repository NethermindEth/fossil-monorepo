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
    pub timestamp: Option<String>,
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

    async fn setup_db() -> Arc<DbConnection> {
        DbConnection::new("postgres://postgres:postgres@localhost:5432")
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_should_get_all_block_headers_by_time_range() {
        let db = setup_db().await;

        let headers = get_block_headers_by_time_range(db, 1743249000, 1743249120)
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
        let db = setup_db().await;

        let headers = get_block_headers_by_time_range(db, 1743249000, 1743249100)
            .await
            .unwrap();

        assert_eq!(headers.len(), 3);
        assert_eq!(headers[0].number, 8006481);
        assert_eq!(headers[1].number, 8006482);
        assert_eq!(headers[2].number, 8006483);
    }

    #[tokio::test]
    async fn test_should_get_block_headers_by_time_range_with_no_results() {
        let db = setup_db().await;

        let headers = get_block_headers_by_time_range(db, 1743248000, 1743249000)
            .await
            .unwrap();

        assert_eq!(headers.len(), 0);
    }

    #[tokio::test]
    async fn test_should_get_all_block_base_fee_by_time_range() {
        let db = setup_db().await;

        let base_fees = get_block_base_fee_by_time_range(db, 1743249000, 1743249120)
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
        let db = setup_db().await;

        let base_fees = get_block_base_fee_by_time_range(db, 1743249000, 1743249100)
            .await
            .unwrap();

        assert_eq!(base_fees.len(), 3);
        assert_eq!(base_fees[0], "0xa0ba15");
        assert_eq!(base_fees[1], "0x9ed346");
        assert_eq!(base_fees[2], "0xa85f1d");
    }

    #[tokio::test]
    async fn test_should_get_block_base_fee_by_time_range_with_no_results() {
        let db = setup_db().await;

        let base_fees = get_block_base_fee_by_time_range(db, 1743248000, 1743249000)
            .await
            .unwrap();

        assert_eq!(base_fees.len(), 0);
    }
}
