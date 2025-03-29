use sqlx::{types::BigDecimal, Error};
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
    start_timestamp: String,
    end_timestamp: String,
) -> Result<Vec<BlockHeader>, Error> {
    tracing::debug!(
        "Getting block headers by time range: {} to {}",
        start_timestamp,
        end_timestamp
    );

    // Parse the strings to i64 before passing to the query
    let start_ts = start_timestamp
        .parse::<i64>()
        .map_err(|e| Error::ColumnDecode {
            index: String::new(),
            source: Box::new(e),
        })?;

    let end_ts = end_timestamp
        .parse::<i64>()
        .map_err(|e| Error::ColumnDecode {
            index: String::new(),
            source: Box::new(e),
        })?;

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
    .bind(start_ts)
    .bind(end_ts)
    .fetch_all(&db.pool)
    .await?;

    Ok(headers)
}

pub async fn get_block_base_fee_by_time_range(
    db: Arc<DbConnection>,
    start_timestamp: String,
    end_timestamp: String,
) -> Result<Vec<BigDecimal>, Error> {
    tracing::debug!(
        "Getting block headers by time range: {} to {}",
        start_timestamp,
        end_timestamp
    );

    // Parse the strings to i64 before passing to the query
    let start_ts = start_timestamp
        .parse::<i64>()
        .map_err(|e| Error::ColumnDecode {
            index: String::new(),
            source: Box::new(e),
        })?;

    let end_ts = end_timestamp
        .parse::<i64>()
        .map_err(|e| Error::ColumnDecode {
            index: String::new(),
            source: Box::new(e),
        })?;

    let base_gas_fees = sqlx::query_scalar(
        r#"
        SELECT CAST(base_fee_per_gas AS NUMERIC)
        FROM blockheaders
        WHERE CAST(timestamp AS BIGINT) BETWEEN $1 AND $2
        ORDER BY number ASC
        "#,
    )
    .bind(start_ts)
    .bind(end_ts)
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

        let headers =
            get_block_headers_by_time_range(db, "1743249000".to_string(), "1743249120".to_string())
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

        let headers =
            get_block_headers_by_time_range(db, "1743249000".to_string(), "1743249100".to_string())
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

        let headers =
            get_block_headers_by_time_range(db, "1743248000".to_string(), "1743249000".to_string())
                .await
                .unwrap();

        assert_eq!(headers.len(), 0);
    }

    #[tokio::test]
    async fn test_should_get_all_block_base_fee_by_time_range() {
        let db = setup_db().await;

        let base_fees = get_block_base_fee_by_time_range(
            db,
            "1743249000".to_string(),
            "1743249120".to_string(),
        )
        .await
        .unwrap();

        assert_eq!(base_fees.len(), 5);
        assert_eq!(
            base_fees[0],
            BigDecimal::from(u64::from_str_radix("a0ba15", 16).unwrap())
        );
        assert_eq!(
            base_fees[1],
            BigDecimal::from(u64::from_str_radix("9ed346", 16).unwrap())
        );
        assert_eq!(
            base_fees[2],
            BigDecimal::from(u64::from_str_radix("a85f1d", 16).unwrap())
        );
        assert_eq!(
            base_fees[3],
            BigDecimal::from(u64::from_str_radix("9aeae1", 16).unwrap())
        );
        assert_eq!(
            base_fees[4],
            BigDecimal::from(u64::from_str_radix("9fda11", 16).unwrap())
        );
    }

    #[tokio::test]
    async fn test_should_only_get_partial_block_base_fee_by_time_range() {
        let db = setup_db().await;

        let base_fees = get_block_base_fee_by_time_range(
            db,
            "1743249000".to_string(),
            "1743249100".to_string(),
        )
        .await
        .unwrap();

        assert_eq!(base_fees.len(), 3);
        assert_eq!(
            base_fees[0],
            BigDecimal::from(u64::from_str_radix("a0ba15", 16).unwrap())
        );
        assert_eq!(
            base_fees[1],
            BigDecimal::from(u64::from_str_radix("9ed346", 16).unwrap())
        );
        assert_eq!(
            base_fees[2],
            BigDecimal::from(u64::from_str_radix("a85f1d", 16).unwrap())
        );
    }

    #[tokio::test]
    async fn test_should_get_block_base_fee_by_time_range_with_no_results() {
        let db = setup_db().await;

        let base_fees = get_block_base_fee_by_time_range(
            db,
            "1743248000".to_string(),
            "1743249000".to_string(),
        )
        .await
        .unwrap();

        assert_eq!(base_fees.len(), 0);
    }
}
