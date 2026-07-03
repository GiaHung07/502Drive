# Recovery Model

Drive writes are effectively-once, not exactly-once. Recovery examines `operation_intents`, destination IDs, and private `appProperties` before retrying an operation.

Cursor advancement for watch is committed in the same transaction as raw change events.

