# Phase 4: Microsoft sign-in (validation in progress)

This branch adds Microsoft sign-in, protected session storage and authenticated play. It is not ready to merge until the live sign-in and game checks below pass.

## Registration and flow

The owner explicitly chose to reuse Kimberlite's Azure client ID: `e38aa735-6b06-4111-9d58-5190f3d754db`. It was recovered from the local Kimberlite core's auth.rs, not substituted from another launcher. The app requests `XboxLive.signin offline_access` from the Microsoft `consumers` device-code endpoint. No client secret, redirect URI, Azure setting change or Kimberlite token-cache reuse.

The registration's current public-client settings, sign-in audience, consent branding and Minecraft API approval have not yet been verified live. Source configuration and prior use are not proof that they still work. Microsoft may show the existing Kimberlite registration name rather than Andromeda.

**Sign in with Microsoft** requests a code only when pressed. The user opens the returned Microsoft verification page and completes consent themselves. Polling honors pending/slow-down, expiry, decline and cancellation. The app exchanges Microsoft to Xbox Live, then XSTS, then Minecraft, verifies Java entitlement and reads the Minecraft profile. Known account/family/registration errors are shown without raw response bodies.

**Use Microsoft account for Play** refreshes and verifies the saved session, then launches with the account's Minecraft profile and access token. It does not fall back silently to offline play. Offline local profiles remain a separate explicit choice. Switching between local and Microsoft identities can change player inventory/position in an existing world because their UUIDs differ.

## Session safety

- Only the refresh token and profile are persisted, protected by Windows current-user DPAPI in `data/auth/session.dpapi` with app-specific entropy. No cleartext-token file or raw auth-body logging.
- The protected cache is separate from Kimberlite. Microsoft/Minecraft access tokens remain in memory only.
- Secret-bearing types have no Debug/Display/Serialize. Launch plans no longer derive Debug because launch arguments can contain an access token.
- Protected writes use a temporary file then replace the old session. Refresh checks that the returned profile matches the saved account.
- **Sign out locally** removes Andromeda's protected file only. It does not sign out Microsoft's browser, revoke consent, or alter Kimberlite.
- Cancellation prevents saving the newly returned session. Closing the app cancels pending polling.

A running Java process receives its access token as a normal Minecraft launch argument. This is not protection against another process running as the same Windows user, nor against a compromised account/device. Refresh-token revocation or an interrupted rotating refresh may require signing in again.

## Verified so far

- Windows fmt, clippy all targets with `-D warnings`, 32 tests.
- Mocked full auth chain, entitlement denial, family/app errors, URI allowlist, pending/slow-down/decline and cancellation.
- Real Windows DPAPI encrypt/decrypt round trip; protected bytes do not contain the test token in cleartext and corrupt bytes fail.
- UI compiled and a separate phase-4 executable linked without replacing the last phase-3 test executable.

## Still required before merge

- Safe-window pixel/interaction inspection of the phase-4 UI, including cancellation and local sign-out.
- Owner-ready real code/consent test using this registration. Verify configured flow/scope and any Microsoft/Minecraft rejection before changing anything.
- Account/profile/skin check, authenticated launch into an isolated test world, close/reopen protected refresh and return-to-ready.
- Regression checks for explicit offline play and existing instances.
- Prior phase proof/merge gates remain in effect. No auto-merge, release or public publication.

Microsoft references checked for this implementation:

- https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-device-code
- https://learn.microsoft.com/en-us/entra/identity-platform/reply-url
- https://learn.microsoft.com/en-us/entra/identity-platform/scenario-desktop-app-configuration
