-- Card payments move from QNB (Pay by Link) to SkipCash. Same flow and table; the columns that
-- named QNB's references get provider-neutral names. (No QNB payment was ever made.)

-- SkipCash's payment id (was QNB's ipgTxnId).
ALTER TABLE payments RENAME COLUMN ipg_txn_id TO provider_payment_id;

-- The card processor's reference SkipCash reports for a paid payment (for reconciliation).
ALTER TABLE payments ADD COLUMN visa_id TEXT;

COMMENT ON COLUMN payments.merchant_txn_id IS 'Our reference, sent to SkipCash as transactionId (max 40)';
COMMENT ON COLUMN payments.last_failure IS 'Why the latest attempt on this payment failed, for the app';
