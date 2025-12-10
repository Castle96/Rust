Apple Music (MusicKit) integration — setup template

Overview

This document shows the minimal steps and a template to integrate Apple Music (MusicKit) into this project.
It covers the required Apple Developer assets, how to generate a developer token (JWT), how to obtain user tokens,
and a small Rust helper template to generate the developer token server-side.

High-level flow

1. Get an Apple Developer account (Organization or Individual) and enable MusicKit.
2. Create a MusicKit private key (.p8) in App Store Connect/Apple Developer (Keys).
3. Generate a Developer Token (JWT) signed with your .p8 private key (server-side). This token is short lived (recommended <= 6 months).
4. Obtain a User Token — typically via MusicKit JS on the client (this requires the developer token) — and send it to your server if you need to call user-scoped API endpoints.
5. Use the Developer Token for Apple Music Web/API calls that require it. Use Developer+User Token pair for user-specific requests.

Required Apple Developer assets

- Team ID (Apple Developer Team ID) — e.g. 1A2BC3D4EF
- Music Key ID (the Key ID shown when you create a MusicKit key) — e.g. ABCDE12345
- Private key file (.p8) downloaded when creating the key — keep it secret

Security notes

- NEVER check private keys or tokens into git.
- Store your private key in a secure place (secret manager, encrypted disk, restricted filesystem permission).
- Shorter Developer Token TTL reduces risk if leaked. Generate on-demand or via a server-side cache.

Environment configuration example

Create a file `config/apple_music.env.example` (DON'T add real secrets) and fill values in your environment management system.

Example `config/apple_music.env.example`:

APPLE_MUSIC_TEAM_ID=YOUR_TEAM_ID
APPLE_MUSIC_KEY_ID=YOUR_KEY_ID
APPLE_MUSIC_PRIVATE_KEY_PATH=/path/to/AuthKey_KEYID.p8
APPLE_MUSIC_DEVELOPER_TOKEN_TTL_SEC=15768000 # 6 months in seconds (or shorter)

Generating a Developer Token (JWT)

The Developer Token is an ES256 JWT with the following claims:
- iss (issuer) = your Team ID
- iat (issued at) = current unix epoch time
- exp (expiration) = iat + TTL (max recommended <= 6 months)
And header includes `kid` = Key ID and alg = ES256.

Example Rust template (server-side) — see `src/playback/applemusic_oauth.rs` in this repo for a runnable template.
The template uses the `jsonwebtoken` crate (or you can use `ring`/`openssl` libs to sign ES256). For security, keep the private key off-repo.

Client-side: Obtaining a User Token

- The recommended method to obtain a user token is via MusicKit JS in a browser. The web client initializes MusicKit with your developer token, then calls `authorizer` to request user authorization and receives a user token.
- Example (MusicKit JS):

```js
MusicKit.configure({
  developerToken: '<DEVELOPER_TOKEN>',
  app: { name: 'MyApp', build: '1.0.0' }
});
const music = MusicKit.getInstance();
const userToken = await music.authorize();
// send userToken to your server if you need user-scoped requests
```

Server-side: Making Apple Music API requests

- Public catalog endpoints often only need the developer token; user-scoped endpoints require the user token as `Music-User-Token` header.
- Example curl call:

curl -H "Authorization: Bearer <DEVELOPER_TOKEN>" -H "Music-User-Token: <USER_TOKEN>" \
  "https://api.music.apple.com/v1/me/library/playlists"

Template integrations included in this repo

- `src/playback/applemusic_oauth.rs`: a small Rust template with a helper to read a .p8 file and build a JWT (ES256). It's a starting point — add `jsonwebtoken` or your preferred JWT library to production code.
- `config/apple_music.env.example`: environment variable example for local development.

Production readiness checklist

- Store `.p8` key securely (secret manager or restricted file). Limit who/what can read it.
- Implement rotation policy and TTL limits for developer tokens.
- Validate user tokens when received and handle token revocation flows.
- Rate limit calls to Apple Music APIs and add exponential backoff on 5xx.
- Add unit and integration tests for your API wrappers (use mocked responses when possible).

If you'd like, I can:
- Add a working Rust implementation that uses `jsonwebtoken` to produce a signed ES256 JWT using an on-disk .p8 (requires adding a dependency).
- Add a small example server endpoint that returns a cached developer token.
- Add a MusicKit JS sample page to obtain a user token for local testing.

Tell me which of those I should implement next and I'll add the code and tests.

