# DEX response captures

Verbatim response bodies from the backend's `APIQueryDexService`
(`anagrambuild/swig-dev-portal` at `85ad6232`), served through its Envoy
transcoder by a local stack on 2026-10-29. Both SDKs' DEX tests read them.

The Swig is `8NkNvqXK6BfLAeqHvcwHWK6b6wybcraj2J3vZHm3LarX` on mainnet, vault
`4k9xWkFvjifracRGbBUHeKg2z8qzJBPwjqjCySu28Mej`. Its 26 Raydium CLMM
transactions are mainnet `getTransaction` responses, archived through
wallet-service's `IngestSwigTransactions`. The backend indexed them itself and
read position state from mainnet. Every request carried an API key of the
organization that owns the Swig.

| File | Request (`...` is `/wallet/api/swig/{swig_config_address}`) |
| --- | --- |
| `list_transactions.json` | `POST .../dex/transactions` `{"network": "NETWORK_MAINNET"}` |
| `list_transactions_filtered.json` | `POST .../dex/transactions` `{"network": "NETWORK_MAINNET", "pageSize": 2, "filters": [{"startSlot": "449000000"}, {"endSlot": "450000000"}, {"protocol": "DEX_PROTOCOL_RAYDIUM_CLMM"}, {"actionType": "DEX_ACTION_TYPE_LIQUIDITY_ADD"}, {"swigVaultAddress": "<vault>"}]}` |
| `list_transactions_filtered_next.json` | `POST .../dex/transactions` `{"network": "NETWORK_MAINNET", "pageSize": 2, "cursor": {"slot": "449761733", "transactionSignature": "1oQ6…"}, "filters": [{"startSlot": "449000000"}, {"endSlot": "450000000"}, {"protocol": "DEX_PROTOCOL_RAYDIUM_CLMM"}, {"actionType": "DEX_ACTION_TYPE_LIQUIDITY_ADD"}, {"swigVaultAddress": "<vault>"}]}` |
| `list_transactions_empty.json` | `POST .../dex/transactions` `{"network": "NETWORK_MAINNET", "filters": [{"actionType": "DEX_ACTION_TYPE_FEE_CLAIM"}]}` |
| `get_transaction_swap.json` | `GET .../dex/transactions/{signature}?network=NETWORK_MAINNET` |
| `list_positions_all.json` | `POST .../dex/positions` `{"network": "NETWORK_MAINNET"}` |
| `list_positions_open.json` | `POST .../dex/positions` `{"network": "NETWORK_MAINNET", "pageSize": 25, "filters": [{"positionStatus": "POSITION_STATUS_OPEN"}, {"protocol": "DEX_PROTOCOL_RAYDIUM_CLMM"}, {"swigVaultAddress": "<vault>"}]}` |
| `list_positions_closed.json` | `POST .../dex/positions` `{"network": "NETWORK_MAINNET", "filters": [{"positionStatus": "POSITION_STATUS_CLOSED"}]}` |
| `list_positions.json` | `POST .../dex/positions` `{"network": "NETWORK_MAINNET", "pageSize": 2}` |
| `list_positions_next.json` | `POST .../dex/positions` `{"network": "NETWORK_MAINNET", "pageSize": 2, "cursor": {"protocol": "DEX_PROTOCOL_RAYDIUM_CLMM", "dexPositionAddress": "BDk8oZeiUPjZMWhpiugc4MY1FvjLgXYu5fxfXGdwMqB7"}}` |
| `list_positions_empty.json` | `POST .../dex/positions` `{"network": "NETWORK_MAINNET", "filters": [{"protocol": "DEX_PROTOCOL_ORCA_WHIRLPOOL"}]}` |

`get_transaction_swap.json` is the transaction `5ic74krw…`. The `cursor`
values were the `nextCursor` of the page before, sent back unchanged.

The history holds no fee claim, position rebalance or reward claim: Raydium
CLMM records a fee claim only for a `DecreaseLiquidityV2` of zero liquidity,
and no venue records the other two yet. Tests build those three from
`swig_dex.proto` instead.
