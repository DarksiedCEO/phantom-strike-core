# Smoke: contracts -> core decision endpoint

Prerequisites
- core running locally
- local env/config present

Run
- `python -m scripts.smoke_contract_decision_submit`

Pass criteria
- valid flat contract payload returns `200`
- invalid payload is rejected
- `trace_id` and `correlation_id` are preserved on success
