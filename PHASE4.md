# Phase 4: Microsoft sign-in (validation in progress)

This branch adds Microsoft sign-in, protected session storage and authenticated Play. Keep this PR draft. The owner's successful sign-in confirmation is not proof of the game, skin or reopen/refresh checks below.

## Registration and flow

The owner selected Kimberlite's public client ID `e38aa735-6b06-4111-9d58-5190f3d754db`. The Azure registration was inspected live on November 10, 2026: its name is Andromeda Launcher, its audience supports personal accounts and organizational tenants, and public-client flows are enabled.

The current UI opens Microsoft's hosted sign-in page in an isolated WebView2 window only when **Sign in with Microsoft** is pressed. It uses authorization code + S256 PKCE, a random state checked on return, and the existing desktop redirect `https://login.microsoftonline.com/common/oauth2/nativeclient`. It requests `XboxLive.signin offline_access` from the consumers endpoint. No client secret, injected JavaScript, credential capture, external popups or Kimberlite token-cache reuse.

An earlier Windows WAM probe failed with ProviderRejected (0x80070525). The owner then requested the standard Microsoft login window. The separately approved broker redirect remains registered but is not used by this WebView2 flow. No account reset was performed. Device-code support remains in the core, but is not currently exposed as a UI fallback.

The code exchanges Microsoft authorization for Xbox Live, XSTS and Minecraft tokens, verifies Java entitlement and reads the Minecraft profile. Known errors do not echo raw response bodies. **Use Microsoft account for Play** refreshes and verifies the protected session and uses the Minecraft profile. It does not silently fall back to offline play. Switching between offline and Microsoft identities can change inventory/position in an existing world because UUIDs differ.

## Session safety

- Only the refresh token and profile are persisted, protected by Windows current-user DPAPI in `data/auth/session.dpapi` with app-specific entropy. Access tokens remain in memory.
- Secret-bearing types have no Debug/Display/Serialize. Launch plans have no Debug because Minecraft launch arguments contain an access token.
- Protected writes use a temporary file then replace the old session. Refresh checks that the returned profile matches the saved account.
- Sign out locally removes this app's protected cache only, not Microsoft consent, browser sessions or Kimberlite data.
- Cancellation prevents saving the returned session. Closing the sign-in window returns control to the launcher.
- Source logging review found no auth code, verifier, state, token, callback URL or raw auth-response logging. A pattern scan of the current application logs returned zero matches for access/refresh-token keys, auth-code/verifier keys, Bearer headers or JWT-like values. This is a bounded check, not a guarantee against every possible secret format.

Minecraft receives its access token as a normal Java launch argument. This does not protect it against other processes running as the same Windows user or a compromised device. Revocation or interrupted rotating refresh may require signing in again.

## Verified

- Windows core tests: 35 passed, including PKCE/callback checks and a real current-user DPAPI round trip. Synthetic auth-chain tests cover entitlement/account errors and cancellation.
- Windows workspace all-target Clippy with `-D warnings` and formatting checks passed.
- A separate test executable was built without replacing the phase-3 `target/debug/andromeda.exe`.
- On November 10, the Sign in crash was traced to a native Windows class-name collision between Slint/winit and Tao. The WebView2 window now has a unique class name.
- The fixed executable's actual Sign in button opened the Microsoft page. Its pixels were inspected. Closing it returned "Sign-in cancelled."; opening it again worked while the launcher stayed alive. No credentials were entered during these automated checks.
- On November 10, 2026, the owner confirmed that sign-in completed with the fixed executable (`Andromeda-Phase4-WebView2-Test/andromeda-fixed.exe`), and that the session was not remembered after reopening. These are owner reports, not proof of the expected skin or a working reopen refresh.
- The protected cache did exist. The startup path had only shown a saved-session notice without loading/refreshing it or restoring the account UI. The new separate `andromeda-restore.exe` silently refreshes it at startup, restores the profile and Microsoft Play choice on success, and keeps the cache on transient errors. Two controlled launches, separated by a normal close, both restored `_4DRI4N_` without a Microsoft window. The actual pixels showed the restored profile and Microsoft Play selected; both processes closed normally.

## Still required before merge

- Owner-reported game/account success covers entering a world with Microsoft Play and remembering the session. Do not substitute owner reports for independent in-world or separate skin evidence.
- Independently check Play with Microsoft in an isolated test world and return to ready; owner-reported skin confirmation exists; an independent in-game inspection remains optional evidence, not an owner-report gap.
- Reopen refresh now passes the two controlled launch checks above. Longer-term expiry/revocation behavior remains covered by synthetic tests rather than a timed live expiry test.
- Check local sign-out and explicit offline-play/instance regressions.
- Keep prior phase-1/intro proof and PR #5/#6/#7 merge gates unchanged. No auto-merge, release or public publication.

References checked:

- https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow
- https://learn.microsoft.com/en-us/entra/msal/dotnet/acquiring-tokens/using-web-browsers
- https://learn.microsoft.com/en-us/entra/identity-platform/reply-url

## Owner follow-up, November 10

In-world skin after Microsoft Play is owner-reported, not an independent screenshot inspection. The stale startup helper that pushed installing 1.21.11 first was replaced with neutral, accurate guidance: choose a profile and Play; installed files are verified before launch. Earlier merge gates remain unchanged.
