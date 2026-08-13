# Matrix Deployment

This directory contains a deploy-ready setup for:

- `arcana.celesteai.ru` -> Arcana client landing page and homeserver entrypoint
- `arcana.celesteai.ru/_matrix/push` -> Sygnal (FCM push gateway for Arcana Android)
- `arcana.celesteai.ru` LiveKit paths (`/rtc`, `/twirp`, `/sfu/get`, `/get_token`, `/healthz`) -> Element Call / MatrixRTC
- `chat.celesteai.ru` -> Element Web
- `call.celesteai.ru` -> self-hosted Element Call (not call.element.io)
- `celesteai.ru/.well-known/matrix/*` -> static Matrix discovery (includes `org.matrix.msc4143.rtc_foci`)

The setup is designed for the current server layout on `celeste`:

- Traefik runs as a system service
- Traefik forwards to localhost services
- the following local ports are already reserved in Traefik:
  - `6167` for Continuwuity
  - `3300` for Element Web
  - `3310` for `.well-known`
  - `3320` for Element Call (`call.celesteai.ru`)
  - `5000` for Sygnal (routed via `arcana.celesteai.ru/_matrix/push`)

## Files

- `docker-compose.yml` - starts the containers
- `continuwuity.toml` - homeserver config
- `element-config.json` - Element Web config
- `element-call-config.json` - self-hosted Element Call config (Arcana homeserver, not Element)
- `well-known/nginx.conf` - static `.well-known` endpoints
- `continuwuity-resolv.conf` - avoids Docker DNS federation issues
- `sygnal/sygnal.yaml` - push gateway config (FCM v1)
- `sygnal/service-account.json` - Firebase Admin SDK key (**not in git**)
- `livekit.yaml` - LiveKit SFU config (**not in git**; copy from `livekit.yaml.example`)

## LiveKit / calls setup

Calls need two things on the server (neither is in git):

1. Repo-root `.env` with matching keys:

```bash
# generate:
docker run --rm livekit/livekit-server:latest generate-keys

# then add to .env:
LIVEKIT_KEY=APIxxxxxxxx
LIVEKIT_SECRET=yyyyyyyy
```

2. A real config file (not a directory):

```bash
# if Docker already created a directory by mistake:
rm -rf deploy/livekit.yaml

cp deploy/livekit.yaml.example deploy/livekit.yaml
# replace LIVEKIT_KEY / LIVEKIT_SECRET in the keys: section with the same values
```

Then restart:

```bash
make up
```

## Element Call (do not use call.element.io)

Stock Element Web opens `https://call.element.io`, which asks users to create an
**Element** account. Arcana hosts its own Call frontend instead.

1. Add DNS: `call.celesteai.ru` -> the same host as `chat.celesteai.ru`.
2. Add a Traefik router: `call.celesteai.ru` -> `127.0.0.1:3320` (TLS like chat).
3. `make up` so the `element-call` container is running.
4. Check:

```bash
curl -I https://call.celesteai.ru
curl -sS https://call.celesteai.ru/config.json
```

`config.json` must name `celesteai.ru` / `https://arcana.celesteai.ru`, not Element.

Web client (`element-config.json`) is already pointed at this URL via `element_call.url`.

## Android push (Firebase / Sygnal)

```
homeserver → https://arcana.celesteai.ru/_matrix/push/v1/notify → FCM → Arcana
```

On the server, place the Firebase service account JSON at
`deploy/sygnal/service-account.json` before starting Sygnal.

## Important

- `server_name` is set to `celesteai.ru`
- changing `server_name` later requires wiping the homeserver database
- user IDs will look like `@user:celesteai.ru`
- email registration is supported by the homeserver once SMTP is configured
- if the client omits `username` during registration, Continuwuity will use the
  verified email localpart as the initial Matrix username

## Registration Model

This deployment is now prepared for email-backed registration:

- registration is enabled
- email is required during registration
- email must be validated before registration completes
- password reset works through email
- users can log in with email if the client sends an email identifier at login

This is a good baseline if you want "email-first" onboarding without MAS.

What it does not do by itself:

- it does not make the Matrix user ID equal to the full email address
- it does not stop all bot signups on its own

In practice, if a user verifies `alice@example.com` and the client does not send
an explicit `username`, the created Matrix ID will default to something like
`@alice:celesteai.ru`.

## SMTP Setup

SMTP credentials are now supplied through the local `.env` file in the
repository root, not committed to git.

1. Copy [`.env.example`](../.env.example) to `../.env` from this directory.
2. Fill in your real Yandex app password.
3. Keep `deploy/continuwuity.toml` unchanged unless you want to change the
   registration policy.

Required variables:

- `CONTINUWUITY_SMTP__CONNECTION_URI`
- `CONTINUWUITY_SMTP__SENDER`
- `CONTINUWUITY_SMTP__REQUIRE_EMAIL_FOR_REGISTRATION`
- `CONTINUWUITY_SMTP__REQUIRE_EMAIL_FOR_TOKEN_REGISTRATION`

Also replace visible product placeholders:

- `"brand": "CHANGE_ME"` in `element-config.json`

The Docker Compose project is pinned by the root `Makefile` as `celesteai` to
keep the existing deployment volume after moving files from `deploy/celesteai/`
to `deploy/`. This is infrastructure naming, not the public messenger name.

## Recommended Hardening

Email verification is better than open registration, but it is still not strong
anti-bot protection by itself. If abuse starts, add one of these:

- enable `suspend_on_register = true` and review new users manually
- add reCAPTCHA in `continuwuity.toml`
- switch back to registration tokens for closed onboarding

## Deploy On Server

Copy this directory to the server, for example:

```bash
scp -r deploy celeste:/root/continuwuity-deploy
```

Then on the server:

```bash
cd ~/continuwuity
make matrix-pull
make matrix-up
make matrix-logs
```

## Verify

After start, check:

```bash
curl -i https://arcana.celesteai.ru/_matrix/client/versions
curl -i https://celesteai.ru/.well-known/matrix/server
curl -i https://celesteai.ru/.well-known/matrix/client
curl -I https://chat.celesteai.ru
curl -I https://arcana.celesteai.ru/privacy
curl -I https://arcana.celesteai.ru/terms
curl -i -X POST https://arcana.celesteai.ru/_matrix/push/v1/notify \
  -H 'Content-Type: application/json' \
  -d '{"notification":{"devices":[]}}'
```

For email flows, also verify:

```bash
curl -i https://arcana.celesteai.ru/_continuwuity/3pid/email/validate
```

Then test from a client:

- request registration email
- click the verification link from the mailbox
- complete registration without sending a separate `username`
- log in using the email address as identifier
- request a password reset email

## Notes

- if you want a support endpoint, add it either in Continuwuity or in `well-known/nginx.conf`
- if you want tighter anti-abuse controls later, combine email verification with
  reCAPTCHA or admin approval
