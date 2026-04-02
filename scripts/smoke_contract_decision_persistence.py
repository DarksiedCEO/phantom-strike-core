from __future__ import annotations

import json
import os
import socket
import subprocess
import tempfile
import time
from pathlib import Path
from urllib import error, parse, request


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


def wait_for_port(host: str, port: int, timeout_seconds: int) -> None:
    deadline = time.time() + timeout_seconds
    while time.time() < deadline:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            sock.settimeout(0.5)
            if sock.connect_ex((host, port)) == 0:
                return
        time.sleep(0.5)

    fail("core did not become reachable", host=host, port=port, timeout_seconds=timeout_seconds)


def http_healthcheck(base_url: str) -> None:
    try:
        with request.urlopen(f"{base_url}/health", timeout=10) as response:
            if response.status != 200:
                fail("health endpoint returned unexpected status", status=response.status)
    except error.URLError as exc:
        fail("health endpoint failed", error=str(exc), base_url=base_url)


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


def start_core(core_base_url: str, decision_store_path: Path) -> subprocess.Popen[str]:
    port = parse.urlparse(core_base_url).port or 80
    env = os.environ.copy()
    env.update(load_dotenv(ENV_PATH))
    env["PORT"] = str(port)
    env["DECISION_STORE_PATH"] = str(decision_store_path)

    process = subprocess.Popen(
        ["cargo", "run"],
        cwd=ROOT,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )

    wait_for_port("127.0.0.1", port, timeout_seconds=30)
    http_healthcheck(core_base_url)
    return process


def stop_core(process: subprocess.Popen[str]) -> None:
    process.terminate()
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=10)


def assert_found_response(envelope: dict, request_payload: dict) -> None:
    if envelope.get("success") is not True:
        fail("retrieval response was not a success envelope", response=envelope)

    data = envelope.get("data")
    if not isinstance(data, dict):
        fail("retrieval envelope missing data payload", response=envelope)

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
                "retrieved decision record mismatch",
                field=key,
                expected=request_payload.get(key),
                actual=data.get(key),
            )


def main() -> int:
    core_base_url = get_setting("CORE_BASE_URL")
    payload = load_json(FIXTURES_DIR / "decision_payload_valid.json")
    lookup_url = f"{core_base_url}/v1/signals/{payload['signal_id']}/decision"

    print(
        json.dumps(
            {
                "stage": "decision_persistence_smoke_start",
                "core_base_url": core_base_url,
                "signal_id": payload["signal_id"],
                "trace_id": payload["trace_id"],
                "correlation_id": payload["correlation_id"],
            },
            indent=2,
            sort_keys=True,
        )
    )

    with tempfile.TemporaryDirectory(prefix="phantom-strike-core-persistence-") as temp_dir:
        decision_store_path = Path(temp_dir) / "signal-decisions.json"

        process = start_core(core_base_url, decision_store_path)
        try:
            submit_status, submit_response = post_json(lookup_url, payload)
            if submit_status != 200:
                fail(
                    "unable to seed durable decision record",
                    status=submit_status,
                    response=submit_response,
                )
        finally:
            stop_core(process)

        restarted = start_core(core_base_url, decision_store_path)
        try:
            get_status, get_response = get_json(
                lookup_url,
                payload["trace_id"],
                payload["correlation_id"],
            )
            if get_status != 200:
                fail(
                    "durably persisted decision record could not be retrieved after restart",
                    status=get_status,
                    response=get_response,
                )

            assert_found_response(get_response, payload)
        finally:
            stop_core(restarted)

    print(
        json.dumps(
            {
                "stage": "decision_persistence_smoke_pass",
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
