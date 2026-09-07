"""Sygnal entrypoint: inject APNs secrets and treat FCM SENDER_ID_MISMATCH as rejected.

Arcana previously used Element's Firebase project (vector-alpha). Tokens from
that project are still stored as pushers. Sending them with the Arcana service
account returns HTTP 403. Upstream Sygnal raises 502 instead of returning the
pushkey in `rejected`, so the homeserver never deletes the stale pusher.

Also expands `${APNS_KEY_ID}` / `${APNS_TEAM_ID}` in sygnal.yaml from `/sygnal/apns.env`
so those values are never committed to git.
"""

from __future__ import annotations

import os
import runpy
import string
from pathlib import Path

import sygnal.gcmpushkin as gcmpushkin

_APNS_ENV_PATH = Path("/sygnal/apns.env")
_SYGNAL_CONF_IN = Path(os.environ.get("SYGNAL_CONF", "/sygnal/sygnal.yaml"))
_SYGNAL_CONF_OUT = Path("/tmp/sygnal.runtime.yaml")


def _load_dotenv(path: Path) -> None:
    if not path.is_file():
        return
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, _, value = line.partition("=")
        key = key.strip()
        value = value.strip().strip("'").strip('"')
        if key and key not in os.environ:
            os.environ[key] = value


def _render_sygnal_conf() -> None:
    _load_dotenv(_APNS_ENV_PATH)
    template = string.Template(_SYGNAL_CONF_IN.read_text(encoding="utf-8"))
    rendered = template.safe_substitute(os.environ)
    _SYGNAL_CONF_OUT.write_text(rendered, encoding="utf-8")
    os.environ["SYGNAL_CONF"] = str(_SYGNAL_CONF_OUT)


_orig_handle_v1 = gcmpushkin.GcmPushkin._handle_v1_response


def _handle_v1_response(self, log, response, response_text, pushkeys, span):
    if response.code == 403:
        log.info(
            "Reg IDs %r get 403 Sender ID mismatch; treating as unregistered. Error: %r",
            pushkeys,
            response_text,
        )
        return pushkeys, []
    return _orig_handle_v1(self, log, response, response_text, pushkeys, span)


gcmpushkin.GcmPushkin._handle_v1_response = _handle_v1_response

_render_sygnal_conf()
runpy.run_module("sygnal.sygnal", run_name="__main__")
