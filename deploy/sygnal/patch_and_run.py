"""Sygnal entrypoint: treat FCM SENDER_ID_MISMATCH as a rejected pushkey.

Arcana previously used Element's Firebase project (vector-alpha). Tokens from
that project are still stored as pushers. Sending them with the Arcana service
account returns HTTP 403. Upstream Sygnal raises 502 instead of returning the
pushkey in `rejected`, so the homeserver never deletes the stale pusher.
"""

from __future__ import annotations

import runpy

import sygnal.gcmpushkin as gcmpushkin

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

runpy.run_module("sygnal.sygnal", run_name="__main__")
