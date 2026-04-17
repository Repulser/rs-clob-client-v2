# Feature Parity: py-clob-client-v2 → rs-clob-client

## Context

The Python CLOB client v2 (`py-clob-client-v2`) and the Rust CLOB client (`polymarket_client_sdk_v2`) target the same Polymarket CLOB REST API. The Rust client is a superset in some areas (WebSocket, Data, Gamma, CTF, Bridge, RTDS modules) but is missing several REST endpoints, RFQ operations, utility functions, and API key management endpoints that the Python client supports. This spec covers closing the gaps so that everything the Python client can do, the Rust client can also do.

## Goal

Achieve full feature parity: every REST endpoint and client-side utility available in `py-clob-client-v2` must have an equivalent in `rs-clob-client`, following existing Rust idioms (type-state auth, `bon` builders, `#[non_exhaustive]`, feature gates).

## Gap Analysis

### Category A — Missing REST API Endpoints

| # | Endpoint | Python Method | HTTP | Path |
|---|----------|---------------|------|------|
| A1 | Read-only API key creation | `create_readonly_api_key()` | `POST` | `/auth/readonly-api-key` |
| A2 | Read-only API keys listing | `get_readonly_api_keys()` | `GET` | `/auth/readonly-api-keys` |
| A3 | Read-only API key deletion | `delete_readonly_api_key(key)` | `DELETE` | `/auth/readonly-api-key` |
| A4 | Market trades events | `get_market_trades_events(condition_id)` | `GET` | `/markets/live-activity/{condition_id}` |
| A5 | Pre-migration orders | `get_pre_migration_orders()` | `GET` | `/data/pre-migration-orders` |
| A6 | CLOB market info | `get_clob_market_info(condition_id)` | `GET` | `/clob-markets/{condition_id}` |
| A7 | Builder fee rates | `__ensure_builder_fee_rate_cached(code)` | `GET` | `/fees/builder-fees/{builder_code}` |
| A8 | Market by token | `__ensure_market_info_cached(token_id)` | `GET` | `/markets-by-token/{token_id}` |

### Category B — Missing RFQ Endpoints (feature: `rfq`)

| # | Endpoint | Python Method | HTTP | Path |
|---|----------|---------------|------|------|
| B1 | RFQ requester quotes | `get_rfq_requester_quotes(params)` | `GET` | `/rfq/data/requester/quotes` |
| B2 | RFQ quoter quotes | `get_rfq_quoter_quotes(params)` | `GET` | `/rfq/data/quoter/quotes` |
| B3 | RFQ best quote | `get_rfq_best_quote(params)` | `GET` | `/rfq/data/best-quote` |
| B4 | RFQ config | `rfq_config()` | `GET` | `/rfq/config` |

**Note on existing `quotes()` method**: The Rust client currently has `quotes()` hitting `/rfq/data/quotes`. The Python client has separate `/rfq/data/requester/quotes` and `/rfq/data/quoter/quotes` endpoints instead. **NEEDS INVESTIGATION**: Does `/rfq/data/quotes` exist server-side? If not, the existing `quotes()` method should be deprecated in favor of B1/B2.

### Category C — Missing Client-Side Utilities

| # | Utility | Python Location | Description |
|---|---------|-----------------|-------------|
| C1 | `calculate_market_price` | `order_builder/builder.py:285` | Walks orderbook depth (reversed) to compute effective fill price for a given amount and order type |
| C2 | `generate_orderbook_summary_hash` | `utilities.py:26` | Compact JSON serialization + SHA1 hash for server-compatible orderbook verification |
| C3 | `adjust_market_buy_amount` | `utilities.py:51` | Adjusts market buy amount to account for platform + builder fees, considering user balance |
| C4 | `price_valid` | `utilities.py` | Validates a price is within [min_tick, 1.0 - min_tick] bounds |

### Category D — Design Differences (informational, no action needed)

| # | Difference | Notes |
|---|-----------|-------|
| D1 | V1 order compatibility | Python supports both V1 and V2 CTF exchange orders. Rust is V2-only. V1 is legacy. **CRITICAL**: Python's `accept_rfq_quote()` and `approve_rfq_order()` build V1 orders. The Rust client already has `accept_quote()` and `approve_order()` methods with V2-style request types — **NEEDS INVESTIGATION**: do these work server-side or are they effectively dead code without V1 support? |
| D2 | Auto-pagination | Python's `get_open_orders()` and `get_trades()` auto-paginate by default. Rust returns `Page<T>` and lets the caller loop. This is idiomatic for Rust — no change needed. |
| D3 | Retry on error | Python has `retry_on_error` config flag. Rust relies on caller-managed retry (idiomatic). No change needed. |
| D4 | Builder fee caching | Python caches builder fee rates per builder_code and uses them in market order construction. Rust should cache this the same way the tick_size/neg_risk/fee_rate caches work. Covered by A7. |

## Design

### A1-A3: Read-only API Key Management

Add to `Client<Authenticated<K>>` (any authenticated kind):

```rust
// src/clob/client.rs — impl<K: Kind> Client<Authenticated<K>>
pub async fn create_readonly_api_key(&self) -> Result<ReadonlyApiKeyResponse>
pub async fn readonly_api_keys(&self) -> Result<Vec<ReadonlyApiKeyResponse>>
pub async fn delete_readonly_api_key(&self, key: &str) -> Result<()>
```

New response type in `src/clob/types/response.rs`:
```rust
#[non_exhaustive]
#[derive(Clone, Debug, Deserialize, Builder, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReadonlyApiKeyResponse {
    pub key: String,
    // NEEDS INVESTIGATION: exact response fields from server
}
```

### A4: Market Trades Events (DEFERRED)

**Deferred** until the response schema for `/markets/live-activity/{condition_id}` is investigated. The Python client returns raw `dict`, and shipping with `serde_json::Value` violates the crate's strong-typing convention. Once the shape is known, add to any-state client with a proper response type.

### A5: Pre-migration Orders

Add to `Client<Authenticated<K>>`:

```rust
pub async fn pre_migration_orders(
    &self,
    next_cursor: Option<String>,
) -> Result<Page<OpenOrderResponse>>
```

Uses existing `OpenOrderResponse` type. Endpoint: `GET /data/pre-migration-orders`.

### A6: CLOB Market Info

Add to any-state client:

```rust
pub async fn clob_market_info(&self, condition_id: &str) -> Result<ClobMarketInfoResponse>
```

New response type:
```rust
#[non_exhaustive]
#[derive(Clone, Debug, Deserialize, Builder, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClobMarketInfoResponse {
    pub condition_id: String,
    pub tokens: Vec<Token>,
    pub min_tick_size: TickSize,
    pub neg_risk: bool,
    pub fee_rate_bps: u32,
    pub fee_exponent: u32,
    // NEEDS INVESTIGATION: exact fields — fee_exponent may be u32 or Decimal
}
```

This also serves as the combined endpoint that Python uses to prime its caches. The Rust implementation should populate:
- `tick_size_cache` (existing `DashMap<U256, TickSize>`)
- `neg_risk_cache` (existing `DashMap<U256, bool>`)
- `fee_rate_cache` (existing `DashMap<U256, FeeRateResponse>`)
- **New** `token_condition_cache` — `DashMap<U256, String>` mapping token_id → condition_id
- **New** `fee_info_cache` — `DashMap<U256, FeeInfo>` storing rate + exponent together

### A7: Builder Fee Rates

Add cached lookup to `Client<Authenticated<K>>` (any authenticated kind, not just Builder):

```rust
pub async fn builder_fee_rate(&self, builder_code: B256) -> Result<BuilderFeeRateResponse>
```

New response type:
```rust
#[non_exhaustive]
#[derive(Clone, Debug, Deserialize, Builder, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BuilderFeeRateResponse {
    /// Maker fee rate as a decimal (server returns BPS, client divides by 10000)
    pub maker: Decimal,
    /// Taker fee rate as a decimal (server returns BPS, client divides by 10000)
    pub taker: Decimal,
}
```

Add a `DashMap<B256, BuilderFeeRateResponse>` cache to `ClientInner`. Must update:
- `ClientInner` struct definition
- `Client::new()` initialization
- `AuthenticationBuilder::authenticate()` — transfer cache
- `promote_to_builder()` — transfer cache
- `invalidate_internal_caches()` — clear cache

### A8: Market by Token (internal helper)

Add as a private/internal method for cache priming:

```rust
async fn market_by_token(&self, token_id: U256) -> Result<MarketResponse>
```

Endpoint: `GET /markets-by-token/{token_id}`. Used internally by `clob_market_info` cache priming to resolve token_id → condition_id.

### B1-B2: RFQ Requester/Quoter Quotes

Add to `Client<Authenticated<K>>` under `#[cfg(feature = "rfq")]`:

```rust
pub async fn requester_quotes(
    &self,
    request: &RfqQuotesRequest,
    next_cursor: Option<&str>,
) -> Result<Page<RfqQuote>>

pub async fn quoter_quotes(
    &self,
    request: &RfqQuotesRequest,
    next_cursor: Option<&str>,
) -> Result<Page<RfqQuote>>
```

These reuse the existing `RfqQuotesRequest` and `RfqQuote` types. Only the URL path differs.

### B3: RFQ Best Quote

```rust
pub async fn best_quote(&self, request_id: &str) -> Result<RfqQuote>
```

Endpoint: `GET /rfq/data/best-quote?requestId={request_id}`.

### B4: RFQ Config

```rust
pub async fn rfq_config(&self) -> Result<RfqConfigResponse>
```

New response type:
```rust
#[cfg(feature = "rfq")]
#[non_exhaustive]
#[derive(Clone, Debug, Deserialize, Builder, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RfqConfigResponse {
    // NEEDS INVESTIGATION: exact fields from server
}
```

### C1: Calculate Market Price

Add as a free function in a new `src/clob/utilities.rs` module:

```rust
/// Walks the orderbook to calculate the effective fill price for a given amount.
///
/// Matches the Python client's behavior:
/// - Iterates positions in reverse order (worst-to-best price levels)
/// - For BUY: accumulates cumulative USDC cost (size * price) until >= amount
/// - For SELL: accumulates cumulative token size until >= amount
/// - For FOK: returns None if insufficient liquidity
/// - For non-FOK (FAK): returns the first available price if any liquidity exists
pub fn calculate_market_price(
    orderbook: &OrderBookSummaryResponse,
    side: Side,
    amount: Decimal,
    order_type: OrderType,
) -> Option<Decimal>
```

### C2: Orderbook Summary Hash

```rust
/// Generates a server-compatible SHA1 hash of an orderbook snapshot.
///
/// Algorithm (must match Python exactly):
/// 1. Construct a JSON object with keys in this exact order:
///    market, asset_id, timestamp, hash (empty string), bids, asks,
///    min_order_size, tick_size, neg_risk, last_trade_price
/// 2. Serialize with compact separators (no spaces): json::to_string
/// 3. SHA1 hash the serialized string
/// 4. Return the hex digest
///
/// NOTE: The existing `OrderBookSummaryResponse::hash()` method uses SHA-256
/// and produces different results. This function is for server-compatible
/// verification. Consider deprecating the existing hash() method.
pub fn orderbook_summary_hash(orderbook: &OrderBookSummaryResponse) -> String
```

Requires adding `sha1 = "0.10"` to `[dependencies]` in Cargo.toml (already a transitive dependency, no new download).

### C3: Adjust Market Buy Amount

```rust
/// Adjusts a market buy USDC amount to account for platform and builder fees.
///
/// Only adjusts when user_usdc_balance <= total cost (amount including fees).
/// Returns the effective amount that can be traded after fees.
///
/// Matches Python's adjust_market_buy_amount from utilities.py:51-74.
pub fn adjust_market_buy_amount(
    amount: Decimal,
    user_usdc_balance: Decimal,
    price: Decimal,
    fee_rate: Decimal,
    fee_exponent: Decimal,
    builder_taker_fee_rate: Decimal,
) -> Decimal
```

### C4: Price Valid

```rust
/// Validates that a price is within the valid range [tick_size, 1 - tick_size].
pub fn price_valid(price: Decimal, tick_size: TickSize) -> bool
```

## Implementation Plan

1. **[File: Cargo.toml]** — Add `sha1 = "0.10"` to `[dependencies]`
2. **[File: src/clob/utilities.rs]** — Create new module with `calculate_market_price`, `orderbook_summary_hash`, `adjust_market_buy_amount`, `price_valid` (C1-C4)
3. **[File: src/clob/mod.rs]** — Add `pub mod utilities;`
4. **[File: src/clob/types/response.rs]** — Add `ReadonlyApiKeyResponse`, `ClobMarketInfoResponse`, `BuilderFeeRateResponse`, `RfqConfigResponse` types with full derive set
5. **[File: src/clob/client.rs]** — Add `DashMap` fields to `ClientInner`: `builder_fee_cache`, `token_condition_cache`, `fee_info_cache`. Update `Client::new()`, `authenticate()`, `promote_to_builder()`, `invalidate_internal_caches()`
6. **[File: src/clob/client.rs]** — Add to `impl<S: State> Client<S>`: `clob_market_info()`, `market_by_token()` (internal)
7. **[File: src/clob/client.rs]** — Add to `impl<K: Kind> Client<Authenticated<K>>`: `create_readonly_api_key()`, `readonly_api_keys()`, `delete_readonly_api_key()`, `pre_migration_orders()`, `builder_fee_rate()`
8. **[File: src/clob/client.rs]** — Add to RFQ `impl<K: Kind> Client<Authenticated<K>>` (feature `rfq`): `requester_quotes()`, `quoter_quotes()`, `best_quote()`, `rfq_config()`

**Deferred**: A4 (market_trades_events) — blocked on response shape investigation.

## Edge Cases

- **Orderbook hash compatibility** → Must exactly match the Python SHA1 implementation (compact JSON serialization with specific key ordering). Port the exact algorithm from `py_clob_client_v2/utilities.py:generate_orderbook_summary_hash`
- **Builder fee cache invalidation** → Follow the same `DashMap` + `invalidate_internal_caches()` pattern as tick_size cache. Add all new caches to the invalidation method
- **Builder fee BPS conversion** → Server returns basis points, client must divide by 10000 to get decimal rate (matching Python behavior)
- **Empty orderbook in calculate_market_price** → Return `None` when insufficient liquidity exists (FOK). For FAK, return first available price if any liquidity
- **Reversed orderbook walk** → Python walks `reversed(positions)` — from worst to best price. Must match this exactly
- **RFQ V1 order requirement** → If the RFQ accept flow requires V1 signed orders, this spec does NOT cover adding V1 order building to Rust. This would be a separate, larger effort

## Open Questions

1. **RFQ V1 dependency**: Does `/rfq/request/accept` require V1 CTF Exchange signed orders? The Python client builds V1 orders. The Rust client has `accept_quote()` with V2-style request types — are these functional server-side or dead code?
2. **Existing `quotes()` vs B1/B2**: Does `/rfq/data/quotes` exist server-side, or only `/rfq/data/requester/quotes` and `/rfq/data/quoter/quotes`? Should existing `quotes()` be deprecated?
3. **`/markets/live-activity/` response shape**: What is the schema? (blocks A4)
4. **`/clob-markets/` response shape**: Exact fields? Is `fee_exponent` a u32 or Decimal?
5. **`/rfq/config` response shape**: What fields?
6. **Read-only API key response shape**: What fields from `POST /auth/readonly-api-key`?
7. **Builder code type**: Is `builder_code` a `B256` in the API path or a hex string?
8. **Existing `OrderBookSummaryResponse::hash()`**: Should this SHA-256 method be deprecated in favor of the server-compatible SHA1 function?

## Verification

- [ ] `cargo check --all-features` passes with no errors
- [ ] `cargo clippy --all-features` passes with no new warnings (no float_arithmetic violations)
- [ ] Unit tests for `calculate_market_price` match Python's `calculate_buy_market_price` / `calculate_sell_market_price` — including reversed walk order and FOK/FAK behavior
- [ ] Unit tests for `orderbook_summary_hash` produce the same hash as Python's `generate_orderbook_summary_hash` for identical input (compact JSON + SHA1)
- [ ] Unit tests for `adjust_market_buy_amount` match Python's test vectors (including builder fee and balance-constrained scenarios)
- [ ] Unit tests for `price_valid` cover tick sizes `0.1`, `0.01`, `0.001`, `0.0001`
- [ ] Integration test: `readonly_api_keys()` returns valid response (requires live API or mock)
- [ ] Integration test: `rfq_config()` returns valid response (requires live API or mock)
- [ ] All new types: `#[non_exhaustive]`, `#[derive(Clone, Debug, Deserialize, Builder, PartialEq)]`, `#[serde(rename_all = "camelCase")]`
- [ ] Builder fee cache: verified at all transfer sites (ClientInner, new(), authenticate(), promote_to_builder(), invalidate)

## Risks

- **SHA1 dependency**: Add `sha1 = "0.10"` crate (already transitive dep, same RustCrypto ecosystem as `sha2`). Not security-critical — content fingerprint only.
- **V1 order gap in RFQ**: If the RFQ accept flow truly requires V1 orders, parity for the RFQ accept path cannot be achieved without a V1 order builder — a significant separate effort.
- **Undocumented endpoints**: Several response shapes marked NEEDS INVESTIGATION require server-side documentation or empirical testing against the live API.
- **Existing hash() incompatibility**: `OrderBookSummaryResponse::hash()` produces SHA-256 hashes, but the server expects SHA1. This may cause confusion if both methods exist.

## Agent Review

| Agent | Verdict | Key Findings |
|-------|---------|-------------|
| claude-architect | APPROVE_WITH_NOTES | C3 missing user_usdc_balance param (critical); C2 hash algorithm wrong; RFQ V1 underplayed; missing /markets-by-token; C1 reversed walk + missing order_type |
| claude-rust-reviewer | APPROVE_WITH_NOTES | Response types missing derives (high); SHA1 crate name wrong; fee_exponent f64 vs clippy lint; builder fee cache transfer sites; builder_fee_rate scope |
