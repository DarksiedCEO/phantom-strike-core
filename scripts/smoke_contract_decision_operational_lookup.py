from __future__ import annotations

import json
import os
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
        fail("fixture missing", path=str(path), error=str(exc))
    except json.JSONDecodeError as exc:
        fail("fixture is invalid json", path=str(path), error=str(exc))


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


def get_json(url: str, trace_id: str, correlation_id: str) -> tuple[int, dict]:
    req = request.Request(
        url,
        headers={
            "x-trace-id": trace_id,
            "x-correlation-id": correlation_id,
        },
        method="GET",
    )

    try:
        with request.urlopen(req, timeout=10) as response:
            return response.status, json.loads(response.read().decode("utf-8"))
    except error.HTTPError as exc:
        return exc.code, json.loads(exc.read().decode("utf-8"))
    except error.URLError as exc:
        fail("unable to connect to core", url=url, error=str(exc))


def assert_found_response(envelope: dict, request_payload: dict, lookup_field: str) -> None:
    if envelope.get("success") is not True:
        fail("lookup response was not a success envelope", lookup_field=lookup_field, response=envelope)

    data = envelope.get("data")
    if not isinstance(data, dict):
        fail("lookup envelope missing data payload", lookup_field=lookup_field, response=envelope)

    for key in (
        "signal_id",
        "baseline_confidence",
        "confidence_delta",
        "updated_confidence",
        "confidence_band",
        "disposition",
        "reasoning",
        "trace_id",
        "correlation_id",
    ):
        if data.get(key) != request_payload.get(key):
            fail(
                "lookup record mismatch",
                lookup_field=lookup_field,
                field=key,
                expected=request_payload.get(key),
                actual=data.get(key),
            )


def main() -> int:
    core_base_url = get_setting("CORE_BASE_URL")
    payload = load_json(FIXTURES_DIR / "decision_payload_valid.json")
    submit_url = f"{core_base_url}/v1/signals/{payload['signal_id']}/decision"
    trace_lookup_url = f"{core_base_url}/v1/decisions/by-trace/{payload['trace_id']}"
    correlation_lookup_url = (
        f"{core_base_url}/v1/decisions/by-correlation/{payload['correlation_id']}"
    )

    print(
        json.dumps(
            {
                "stage": "decision_operational_lookup_smoke_start",
                "core_base_url": core_base_url,
                "signal_id": payload["signal_id"],
                "trace_id": payload["trace_id"],
                "correlation_id": payload["correlation_id"],
            },
            indent=2,
            sort_keys=True,
        )
    )

    submit_status, submit_response = post_json(submit_url, payload)
    if submit_status != 200:
        fail("unable to seed decision record for operational lookup smoke", status=submit_status, response=submit_response)

    trace_status, trace_response = get_json(
        trace_lookup_url,
        payload["trace_id"],
        payload["correlation_id"],
    )
    if trace_status != 200:
        fail("decision record could not be retrieved by trace_id", status=trace_status, response=trace_response)
    assert_found_response(trace_response, payload, "trace_id")

    correlation_status, correlation_response = get_json(
        correlation_lookup_url,
        payload["trace_id"],
        payload["correlation_id"],
    )
    if correlation_status != 200:
        fail(
            "decision record could not be retrieved by correlation_id",
            status=correlation_status,
            response=correlation_response,
        )
    assert_found_response(correlation_response, payload, "correlation_id")

    print(
        json.dumps(
            {
                "stage": "decision_operational_lookup_smoke_pass",
                "submit_status": submit_status,
                "trace_status": trace_status,
                "correlation_status": correlation_status,
                "signal_id": payload["signal_id"],
                "trace_id": payload["trace_id"],
                "correlation_id": payload["correlation_id"],
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
