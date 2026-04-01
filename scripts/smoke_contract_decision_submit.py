from __future__ import annotations

import json
import os
import sys
from pathlib import Path
from urllib import error, request


ROOT = Path(__file__).resolve().parents[1]
FIXTURES_DIR = ROOT / "tests" / "fixtures"
ENV_PATH = ROOT / ".env"
ENV_EXAMPLE_PATH = ROOT / ".env.example"


def fail(message: str, **details: object) -> None:
    print(f"SMOKE FAIL: {message}")
    if details:
        print(json.dumps(details, indent=2, sort_keys=True, default=str))
    raise SystemExit(1)


def load_dotenv(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    if not path.exists():
        return values

    for raw_line in path.read_text().splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip().strip("\"").strip("'")
    return values


def get_setting(name: str) -> str:
    env_values = load_dotenv(ENV_PATH)
    if name in os.environ:
        return os.environ[name]
    if name in env_values:
        return env_values[name]
    fail(
        f"missing required setting {name}",
        env_file=str(ENV_PATH),
        env_example=str(ENV_EXAMPLE_PATH),
    )


def load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text())
    except FileNotFoundError as exc:
        fail("fixture or schema file missing", path=str(path), error=str(exc))
    except json.JSONDecodeError as exc:
        fail("fixture or schema file is invalid json", path=str(path), error=str(exc))


def load_schema(schema_dir: Path, file_name: str) -> dict:
    document = load_json(schema_dir / file_name)
    reference = document["$ref"].split("/")[-1]
    return document["definitions"][reference]


def assert_contract_shape(payload: dict, schema: dict, context: str) -> None:
    required = set(schema["required"])
    actual = set(payload.keys())
    if actual != required:
        fail(
            f"{context} key mismatch",
            expected=sorted(required),
            actual=sorted(actual),
            payload=payload,
        )

    enum_fields = {
        field_name: field_schema["enum"]
        for field_name, field_schema in schema["properties"].items()
        if isinstance(field_schema, dict) and "enum" in field_schema
    }

    for field_name, allowed in enum_fields.items():
        if payload[field_name] not in allowed:
            fail(
                f"{context} enum mismatch",
                field=field_name,
                allowed=allowed,
                actual=payload[field_name],
            )


def post_json(url: str, payload: dict) -> tuple[int, dict]:
    body = json.dumps(payload).encode("utf-8")
    req = request.Request(
        url,
        data=body,
        headers={
            "content-type": "application/json",
            "x-trace-id": payload["trace_id"],
            "x-correlation-id": payload["correlation_id"],
        },
        method="POST",
    )

    try:
        with request.urlopen(req, timeout=10) as response:
            return response.status, json.loads(response.read().decode("utf-8"))
    except error.HTTPError as exc:
        return exc.code, json.loads(exc.read().decode("utf-8"))
    except error.URLError as exc:
        fail("unable to connect to core", url=url, error=str(exc))


def assert_success_response(envelope: dict, request_payload: dict, response_schema: dict) -> None:
    if envelope.get("success") is not True:
        fail("response shape invalid", response=envelope)

    data = envelope.get("data")
    meta = envelope.get("meta")
    if not isinstance(data, dict) or not isinstance(meta, dict):
        fail("response envelope missing data or meta", response=envelope)

    assert_contract_shape(data, response_schema, "response data")

    if data["trace_id"] != request_payload["trace_id"]:
        fail(
            "trace_id mismatch",
            expected=request_payload["trace_id"],
            actual=data["trace_id"],
        )
    if data["correlation_id"] != request_payload["correlation_id"]:
        fail(
            "correlation_id mismatch",
            expected=request_payload["correlation_id"],
            actual=data["correlation_id"],
        )

    if meta.get("trace_id") != request_payload["trace_id"]:
        fail(
            "response meta trace_id mismatch",
            expected=request_payload["trace_id"],
            actual=meta.get("trace_id"),
        )
    if meta.get("correlation_id") != request_payload["correlation_id"]:
        fail(
            "response meta correlation_id mismatch",
            expected=request_payload["correlation_id"],
            actual=meta.get("correlation_id"),
        )


def assert_validation_failure(envelope: dict) -> None:
    if envelope.get("success") is not False:
        fail("invalid payload response was not a failure envelope", response=envelope)

    error_body = envelope.get("error")
    if not isinstance(error_body, dict):
        fail("invalid payload response missing error body", response=envelope)

    if error_body.get("code") != "CONTRACT_VALIDATION_FAILED":
        fail(
            "invalid payload returned unexpected error code",
            expected="CONTRACT_VALIDATION_FAILED",
            actual=error_body.get("code"),
            response=envelope,
        )


def main() -> int:
    core_base_url = get_setting("CORE_BASE_URL")
    contracts_schema_dir = Path(get_setting("CONTRACTS_SCHEMA_DIR"))

    request_schema = load_schema(contracts_schema_dir, "signalDecision.json")
    response_schema = load_schema(contracts_schema_dir, "decisionSubmissionResult.json")

    valid_payload = load_json(FIXTURES_DIR / "decision_payload_valid.json")
    invalid_payload = load_json(FIXTURES_DIR / "decision_payload_invalid.json")

    assert_contract_shape(valid_payload, request_schema, "valid request")

    print(
        json.dumps(
            {
                "stage": "contract_smoke_start",
                "core_base_url": core_base_url,
                "contracts_schema_dir": str(contracts_schema_dir),
                "valid_signal_id": valid_payload["signal_id"],
                "invalid_signal_id": invalid_payload["signal_id"],
            },
            indent=2,
            sort_keys=True,
        )
    )

    valid_status, valid_response = post_json(
        f"{core_base_url}/v1/signals/{valid_payload['signal_id']}/decision",
        valid_payload,
    )
    if valid_status != 200:
        fail(
            "valid payload rejected",
            status=valid_status,
            response=valid_response,
        )
    assert_success_response(valid_response, valid_payload, response_schema)

    invalid_status, invalid_response = post_json(
        f"{core_base_url}/v1/signals/{invalid_payload['signal_id']}/decision",
        invalid_payload,
    )
    if invalid_status == 200:
        fail("invalid payload was accepted", response=invalid_response)
    assert_validation_failure(invalid_response)

    print(
        json.dumps(
            {
                "stage": "contract_smoke_pass",
                "valid_status": valid_status,
                "invalid_status": invalid_status,
                "trace_id": valid_payload["trace_id"],
                "correlation_id": valid_payload["correlation_id"],
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
