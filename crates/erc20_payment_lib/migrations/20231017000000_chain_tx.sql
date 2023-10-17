CREATE UNIQUE INDEX "chain_tx_tx_hash" ON "chain_tx" (tx_hash);

ALTER TABLE "chain_transfer" ADD COLUMN "fee_paid" TEXT NULL;
ALTER TABLE "chain_transfer" ADD COLUMN "blockchain_date" DATETIME NULL;

