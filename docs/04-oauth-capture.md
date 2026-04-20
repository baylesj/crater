# Capturing your SoundCloud OAuth token

To export playlists, crater needs to act as you on SoundCloud. The
unofficial v2 API has no programmatic auth flow (no OAuth app
registration, no consent screen) — the only way to get a working token
is to scrape it from your own logged-in browser session.

This is a one-time manual step. The token lasts **about a year** before
SoundCloud rotates it, at which point you redo this.

## The flow (Chrome / Firefox, ~2 minutes)

1. **Log in** at <https://soundcloud.com>. Make sure you're signed into
   the account you want crater to manage playlists on.

2. **Open DevTools** (Cmd+Opt+I on macOS, Ctrl+Shift+I elsewhere). Go
   to the **Network** tab.

3. **Filter to API calls.** In the filter box, type `api-v2` to narrow
   requests to the SoundCloud internal API.

4. **Trigger a request.** Click anywhere on soundcloud.com — your
   feed, a track, whatever. You'll see requests appear in the network
   panel.

5. **Pick any request** with the path `api-v2.soundcloud.com/...` and
   click it. In the right pane, go to **Headers** → **Request Headers**.

6. **Find `Authorization`.** The value looks like:
   ```
   Authorization: OAuth 2-294714-12345678-abcdEfGhIjKlMnOp
   ```
   Everything after `OAuth ` is the token. Copy the whole value
   (with or without the `OAuth ` prefix — crater accepts both).

7. **Paste into crater.** Either:
   - Settings page → SoundCloud token → paste → save, or
   - Set the env var on your Docker container:
     ```
     CRATER_SC_OAUTH_TOKEN=2-294714-12345678-abcdEfGhIjKlMnOp
     ```
     and restart.

## How crater stores it

- In the DB: hash only. The full token goes to the OS keyring on Mac /
  to a 0600-permissioned file on Linux (`$CRATER_DATA_DIR/oauth.token`).
- In memory: plain, as long as the process is running.
- In logs: never — tracing spans redact anything matching the OAuth
  token pattern.
- In the UI: only the last 6 characters ever displayed
  (`***abcdEfGh`), with a "rotate" button that clears it.

Docker caveat: if you pass it via env var (`CRATER_SC_OAUTH_TOKEN`),
it's visible to anyone who can `docker inspect` the container. If that
matters (shared Unraid, multiple users), use the settings-page flow and
let crater manage the keyring/file instead.

## Verifying the token works

Settings page has a "test token" button that hits
`https://api-v2.soundcloud.com/me` with the token. Returns your SC
username on success, a specific error on failure:
- 401 → token is bad or expired; repeat the capture flow
- other → something else broke, check logs

## When it stops working

Symptoms:
- Digest runs fail with `sc_client::error: AuthExpired` on the playlist
  create step (search still works — that uses `client_id`, not OAuth)
- `/api/playlists/export` returns 401 with message "SoundCloud rejected
  OAuth token"
- Settings page "test token" fails with 401

Action: redo the 7-step capture flow above, paste new token. Crater
does not retry automatically because the token has to come from a
human-driven browser session.

Proactive rotation: set a calendar reminder for ~10 months after
capture. Doing this before expiry avoids a missed digest run.

## Security posture

The OAuth token gives full account access: create/delete playlists,
follow/unfollow, post comments, change profile. Crater only uses it for
playlist CRUD, but a compromised crater instance could do any of those.

Mitigations:
- Keep crater on your LAN only. Don't expose it to the internet without
  additional auth (Cloudflare Access, Tailscale, etc.).
- If you suspect compromise: log out of all sessions on SoundCloud
  (Settings → Security → Sign out of all devices). This invalidates
  the token.
- If the Unraid box is compromised, the token is gone anyway — treat
  OS-level security as the actual perimeter.

## If you want to avoid this step

Apply for an official SoundCloud API key
(<https://developers.soundcloud.com/>). Non-commercial personal tools
are the use case they approve. Approval takes anywhere from days to
months. Once approved, crater can swap to the official OAuth flow with
a consent screen — swap is a `sc_client` feature flag, the rest of the
codebase doesn't change.

Current status: TODO, not yet applied. Worth doing in parallel with
building the tool.
