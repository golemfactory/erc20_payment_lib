# rust_erc20_payment


# Notes

* all addresses/txs in database are in hex/lowercase format. 

# Frontend

Link to frontend repo:
https://github.com/scx1332/erc20_driver_frontend

# Sample environment setup

ETH_PRIVATE_KEY=0000000000000000000000000000000000000000000000000000000000000000
PROVIDER_URL=https://rpc-mumbai.matic.today
RUST_LOG=debug,sqlx::query=warn,web=warn

# Sample runs

```
cargo run -- transfer --plain-eth --amounts=1,2,3,4 --receivers=0xA000000000000000000000000000000000050001,0xA000000000000000000000000000000000050002,0xa000000000000000000000000000000000050003,0xa000000000000000000000000000000000050004
cargo run -- transfer --token-addr=0x2036807b0b3aaf5b1858ee822d0e111fddac7018 --amounts=1,2,3,4 --receivers=0xA000000000000000000000000000000000050001,0xA000000000000000000000000000000000050002,0xa000000000000000000000000000000000050003,0xa000000000000000000000000000000000050004
cargo run --example generate_transfers -- --chain-name dev --address-pool-size 10000 --amounts-pool-size 10000 --generate-count 100
```

prepare test transfers into db, it generates 100 random GLM transfers to 10 unique addresses

```cargo run --example generate_transfers -- --generate-count 100 --address-pool-size 10 --amounts-pool-size=100```

dry run without processing transactions

```cargo run -- process --generate-tx-only=1```

Useful command to see transactions being processed
```sql
SELECT id,
       (CAST((julianday(broadcast_date) - 2440587.5)*86400000 AS INTEGER) - CAST((julianday(created_date) - 2440587.5)*86400000 AS INTEGER)) / 1000.0 as broadcast_delay,
       broadcast_count,
       (CAST((julianday(confirm_date) - 2440587.5)*86400000 AS INTEGER) - CAST((julianday(broadcast_date) - 2440587.5)*86400000 AS INTEGER)) / 1000.0 as confirm_delay,
       tx_hash,
       *
FROM tx
order by created_date desc
```

Clean all transactions and transfers
```sql
DELETE FROM token_transfer;
DELETE FROM tx;
```

# TODO

- [x] Add error handling in gather_transactions, now SQL will loop forever, when hit error in gather


