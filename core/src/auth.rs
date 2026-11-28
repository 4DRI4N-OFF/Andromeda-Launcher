//! Microsoft device-code sign-in. Tokens are never formatted, logged or serialized.
use serde_json::{json, Value};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub const CLIENT_ID: &str = "e38aa735-6b06-4111-9d58-5190f3d754db";
const SCOPES: &str = "XboxLive.signin offline_access";
const DEVICE: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const ENTITLEMENTS: &str = "https://api.minecraftservices.com/entitlements/mcstore";
const PROFILE: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthError {
    Network,
    InvalidResponse,
    Rejected,
    Cancelled,
    Expired,
    Declined,
    InvalidSession,
    NoXboxProfile,
    XboxUnavailable,
    AdultVerification,
    FamilyRequired,
    AppNotApproved,
    NoGame,
    NoProfile,
    Storage,
}
impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Network => "Could not contact the sign-in service. Try again.",
            Self::InvalidResponse => "The sign-in service returned an invalid response.",
            Self::Rejected => "Microsoft or Xbox rejected this sign-in request.",
            Self::Cancelled => "Sign-in cancelled.",
            Self::Expired => "Sign-in code expired. Start again.",
            Self::Declined => "Sign-in was declined on the Microsoft page.",
            Self::InvalidSession => "Saved sign-in is no longer valid. Sign in again.",
            Self::NoXboxProfile => "Create an Xbox profile for this Microsoft account first.",
            Self::XboxUnavailable => "Xbox Live is not available for this account's country.",
            Self::AdultVerification => "Xbox requires adult verification for this account.",
            Self::FamilyRequired => {
                "This account needs a Microsoft family setup before Xbox sign-in."
            }
            Self::AppNotApproved => {
                "Minecraft rejected the app registration. Its API access needs checking."
            }
            Self::NoGame => "This account does not have a Minecraft Java entitlement.",
            Self::NoProfile => "Create a Minecraft Java profile on minecraft.net first.",
            Self::Storage => "Could not read or protect the local sign-in session.",
        })
    }
}
impl std::error::Error for AuthError {}
type Result<T> = std::result::Result<T, AuthError>;

// No Debug, Display, Clone or Serialize for secret-bearing values.
pub struct Secret(String);
impl Secret {
    pub(crate) fn new(value: String) -> Self {
        Self(value)
    }
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    interval: u64,
    started: Instant,
    device_code: Secret,
}
pub struct Session {
    pub profile: Profile,
    pub(crate) refresh_token: Secret,
    pub(crate) minecraft_token: Secret,
    pub(crate) expires_at: u64,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
}
pub struct Reply {
    pub status: u16,
    pub body: Value,
}
pub trait Transport {
    fn form(&self, url: &str, fields: &[(&str, &str)]) -> Result<Reply>;
    fn json(&self, url: &str, body: Value) -> Result<Reply>;
    fn bearer(&self, url: &str, token: &str) -> Result<Reply>;
}
pub struct Http(reqwest::blocking::Client);
impl Http {
    pub fn new() -> Result<Self> {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(25))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(format!("Andromeda/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map(Self)
            .map_err(|_| AuthError::Network)
    }
    fn read(
        response: std::result::Result<reqwest::blocking::Response, reqwest::Error>,
    ) -> Result<Reply> {
        use std::io::Read;
        let r = response.map_err(|_| AuthError::Network)?;
        let status = r.status().as_u16();
        let mut bytes = Vec::new();
        r.take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| AuthError::Network)?;
        if bytes.len() > 1024 * 1024 {
            return Err(AuthError::InvalidResponse);
        }
        let body = serde_json::from_slice(&bytes).map_err(|_| AuthError::InvalidResponse)?;
        Ok(Reply { status, body })
    }
}
impl Transport for Http {
    fn form(&self, u: &str, f: &[(&str, &str)]) -> Result<Reply> {
        Self::read(self.0.post(u).form(f).send())
    }
    fn json(&self, u: &str, b: Value) -> Result<Reply> {
        Self::read(self.0.post(u).json(&b).send())
    }
    fn bearer(&self, u: &str, t: &str) -> Result<Reply> {
        Self::read(self.0.get(u).bearer_auth(t).send())
    }
}
fn text(v: &Value, key: &str) -> Result<String> {
    v[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or(AuthError::InvalidResponse)
}
fn success(r: Reply) -> Result<Value> {
    if r.status == 200 {
        Ok(r.body)
    } else {
        Err(AuthError::Rejected)
    }
}
/// Start only after a user chooses Sign in. No client secret or redirect URI.
pub fn begin(http: &dyn Transport) -> Result<DeviceCode> {
    let v = success(http.form(DEVICE, &[("client_id", CLIENT_ID), ("scope", SCOPES)])?)?;
    let uri = text(&v, "verification_uri")?;
    if uri != "https://www.microsoft.com/link"
        && uri != "https://microsoft.com/devicelogin"
        && uri != "https://www.microsoft.com/devicelogin"
    {
        return Err(AuthError::InvalidResponse);
    }
    let expires_in = v["expires_in"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= 1800)
        .ok_or(AuthError::InvalidResponse)?;
    let interval = v["interval"].as_u64().unwrap_or(5).clamp(1, 60);
    let user_code = text(&v, "user_code")?;
    if user_code.len() > 32
        || !user_code
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(AuthError::InvalidResponse);
    }
    Ok(DeviceCode {
        user_code,
        verification_uri: uri,
        expires_in,
        interval,
        started: Instant::now(),
        device_code: Secret(text(&v, "device_code")?),
    })
}
fn token_reply(r: Reply) -> Result<(Secret, Secret)> {
    let v = success(r)?;
    Ok((
        Secret(text(&v, "access_token")?),
        Secret(text(&v, "refresh_token")?),
    ))
}
#[derive(Debug, PartialEq)]
enum Poll {
    Pending,
    SlowDown,
    Stop(AuthError),
    Ready,
}
fn poll_state(r: &Reply) -> Poll {
    if r.status == 200 {
        return Poll::Ready;
    }
    match r.body["error"].as_str() {
        Some("authorization_pending") => Poll::Pending,
        Some("slow_down") => Poll::SlowDown,
        Some("authorization_declined" | "access_denied") => Poll::Stop(AuthError::Declined),
        Some("expired_token" | "bad_verification_code") => Poll::Stop(AuthError::Expired),
        _ => Poll::Stop(AuthError::Rejected),
    }
}
pub fn finish(http: &dyn Transport, code: &DeviceCode, cancel: &AtomicBool) -> Result<Session> {
    let mut interval = code.interval;
    loop {
        for _ in 0..interval * 4 {
            if cancel.load(Ordering::Relaxed) {
                return Err(AuthError::Cancelled);
            }
            if code.started.elapsed().as_secs() >= code.expires_in {
                return Err(AuthError::Expired);
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        let r = http.form(
            TOKEN,
            &[
                ("client_id", CLIENT_ID),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", code.device_code.expose()),
            ],
        )?;
        match poll_state(&r) {
            Poll::Pending => continue,
            Poll::SlowDown => {
                interval = interval.saturating_add(5);
                continue;
            }
            Poll::Stop(e) => return Err(e),
            Poll::Ready => {
                let (access, refresh) = token_reply(r)?;
                if cancel.load(Ordering::Relaxed) {
                    return Err(AuthError::Cancelled);
                }
                return exchange(http, access, refresh);
            }
        }
    }
}
pub fn refresh(http: &dyn Transport, token: &Secret) -> Result<Session> {
    let r = http.form(
        TOKEN,
        &[
            ("client_id", CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", token.expose()),
            ("scope", SCOPES),
        ],
    )?;
    if r.status != 200 {
        return Err(AuthError::InvalidSession);
    }
    let access = Secret(text(&r.body, "access_token")?);
    let refresh = Secret(
        r.body["refresh_token"]
            .as_str()
            .unwrap_or(token.expose())
            .to_owned(),
    );
    exchange(http, access, refresh)
}
fn exchange(http: &dyn Transport, access: Secret, refresh: Secret) -> Result<Session> {
    let xbl=success(http.json(XBL,json!({"Properties":{"AuthMethod":"RPS","SiteName":"user.auth.xboxlive.com","RpsTicket":format!("d={}",access.expose())},"RelyingParty":"http://auth.xboxlive.com","TokenType":"JWT"}))?)?;
    let hash = xbl["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .ok_or(AuthError::InvalidResponse)?;
    let xsts=http.json(XSTS,json!({"Properties":{"SandboxId":"RETAIL","UserTokens":[text(&xbl,"Token")?]},"RelyingParty":"rp://api.minecraftservices.com/","TokenType":"JWT"}))?;
    if xsts.status != 200 {
        return Err(match xsts.body["XErr"].as_u64() {
            Some(2148916233) => AuthError::NoXboxProfile,
            Some(2148916235) => AuthError::XboxUnavailable,
            Some(2148916236 | 2148916237) => AuthError::AdultVerification,
            Some(2148916238) => AuthError::FamilyRequired,
            _ => AuthError::Rejected,
        });
    }
    let mc = http.json(
        MC,
        json!({"identityToken":format!("XBL3.0 x={};{}",hash,text(&xsts.body,"Token")?)}),
    )?;
    if mc.status == 403 {
        return Err(AuthError::AppNotApproved);
    }
    let mc = success(mc)?;
    let token = Secret(text(&mc, "access_token")?);
    let entitlements = success(http.bearer(ENTITLEMENTS, token.expose())?)?;
    if !entitlements["items"]
        .as_array()
        .ok_or(AuthError::InvalidResponse)?
        .iter()
        .any(|v| {
            matches!(
                v["name"].as_str(),
                Some("game_minecraft" | "product_minecraft")
            )
        })
    {
        return Err(AuthError::NoGame);
    }
    let profile = http.bearer(PROFILE, token.expose())?;
    if profile.status == 404 {
        return Err(AuthError::NoProfile);
    }
    let p = success(profile)?;
    let id = text(&p, "id")?;
    let name = text(&p, "name")?;
    if id.len() != 32
        || !id.bytes().all(|b| b.is_ascii_hexdigit())
        || crate::launch::offline_uuid(&name).is_err()
    {
        return Err(AuthError::InvalidResponse);
    }
    let ttl = mc["expires_in"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= 172800)
        .ok_or(AuthError::InvalidResponse)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| AuthError::InvalidResponse)?
        .as_secs();
    Ok(Session {
        profile: Profile { id, name },
        refresh_token: refresh,
        minecraft_token: token,
        expires_at: now + ttl,
    })
}
impl Session {
    pub fn is_expired(&self) -> bool {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().saturating_add(60) >= self.expires_at)
            .unwrap_or(true)
    }
    pub(crate) fn access_token(&self) -> &str {
        self.minecraft_token.expose()
    }
}
pub(crate) fn finish_authorization_code(http: &dyn Transport, r: Reply) -> Result<Session> {
    let (access, refresh) = token_reply(r)?;
    exchange(http, access, refresh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    struct Mock(Mutex<std::collections::VecDeque<Reply>>);
    impl Mock {
        fn next(&self) -> Result<Reply> {
            self.0.lock().unwrap().pop_front().ok_or(AuthError::Network)
        }
    }
    impl Transport for Mock {
        fn form(&self, _: &str, _: &[(&str, &str)]) -> Result<Reply> {
            self.next()
        }
        fn json(&self, _: &str, _: Value) -> Result<Reply> {
            self.next()
        }
        fn bearer(&self, _: &str, _: &str) -> Result<Reply> {
            self.next()
        }
    }
    fn mock(replies: Vec<(u16, Value)>) -> Mock {
        Mock(Mutex::new(
            replies
                .into_iter()
                .map(|(status, body)| Reply { status, body })
                .collect(),
        ))
    }
    fn chain() -> Vec<(u16, Value)> {
        vec![
            (
                200,
                json!({"Token":"test-xbl","DisplayClaims":{"xui":[{"uhs":"test-hash"}]}}),
            ),
            (200, json!({"Token":"test-xsts"})),
            (200, json!({"access_token":"test-mc","expires_in":3600})),
            (200, json!({"items":[{"name":"game_minecraft"}]})),
            (
                200,
                json!({"id":"0123456789abcdef0123456789abcdef","name":"Player"}),
            ),
        ]
    }
    #[test]
    fn successful_chain_requires_entitlement_and_profile() {
        let s = exchange(
            &mock(chain()),
            Secret("test".into()),
            Secret("refresh".into()),
        )
        .unwrap();
        assert_eq!(s.profile.name, "Player");
        assert_eq!(s.minecraft_token.expose(), "test-mc");
        assert_eq!(s.refresh_token.expose(), "refresh");
        assert!(s.expires_at > 0);
    }
    #[test]
    fn no_entitlement_blocks_profile() {
        let mut c = chain();
        c[3] = (200, json!({"items":[]}));
        assert!(matches!(
            exchange(&mock(c), Secret("a".into()), Secret("r".into())),
            Err(AuthError::NoGame)
        ));
    }
    #[test]
    fn family_and_app_errors_are_safe() {
        let mut c = chain();
        c[1] = (401, json!({"XErr":2148916238u64}));
        assert!(matches!(
            exchange(&mock(c), Secret("a".into()), Secret("r".into())),
            Err(AuthError::FamilyRequired)
        ));
        let mut c = chain();
        c[2] = (403, json!({"secret":"never display"}));
        assert!(matches!(
            exchange(&mock(c), Secret("a".into()), Secret("r".into())),
            Err(AuthError::AppNotApproved)
        ));
    }
    #[test]
    fn device_uri_is_allowlisted() {
        let h = mock(vec![(
            200,
            json!({"verification_uri":"https://attacker.invalid","user_code":"12345","device_code":"test","expires_in":900}),
        )]);
        assert!(matches!(begin(&h), Err(AuthError::InvalidResponse)));
    }
    #[test]
    fn poll_handles_pending_slowdown_and_decline() {
        assert_eq!(
            poll_state(&Reply {
                status: 400,
                body: json!({"error":"slow_down"})
            }),
            Poll::SlowDown
        );
        assert_eq!(
            poll_state(&Reply {
                status: 400,
                body: json!({"error":"authorization_pending"})
            }),
            Poll::Pending
        );
        assert_eq!(
            poll_state(&Reply {
                status: 400,
                body: json!({"error":"authorization_declined"})
            }),
            Poll::Stop(AuthError::Declined)
        );
    }
    #[test]
    fn cancelled_flow_does_not_poll() {
        let code = DeviceCode {
            user_code: "TEST".into(),
            verification_uri: "https://www.microsoft.com/link".into(),
            expires_in: 900,
            interval: 5,
            started: Instant::now(),
            device_code: Secret("test".into()),
        };
        assert!(matches!(
            finish(&mock(vec![]), &code, &AtomicBool::new(true)),
            Err(AuthError::Cancelled)
        ));
    }
}
