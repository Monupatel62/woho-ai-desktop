# Training progress contract

Runtime progress lines are newline-delimited JSON on stdout.

Schema version: 1.

Example:

{"status":"progress","jobId":"job-123","phase":"validation","completed":1,"total":1,"percent":100}

Consumers must treat unknown fields as forward-compatible and must reject invalid percentages outside 0..100.
