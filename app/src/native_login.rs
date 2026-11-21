//! Microsoft's hosted sign-in page in an isolated WebView2 window.
//! No injected JavaScript, credential access, URL logging, or external windows.
use andromeda_core::auth::{AuthError, Session};
use std::sync::atomic::AtomicBool;
#[cfg(target_os = "windows")]
pub fn sign_in(cancel: &AtomicBool) -> Result<Session, AuthError> {
    use andromeda_core::{
        auth::Http,
        pkce::{allowed_navigation, Callback, Pending},
    };
    use std::{
        sync::{atomic::Ordering, Arc, Mutex},
        time::{Duration, Instant},
    };
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoopBuilder},
        platform::{
            run_return::EventLoopExtRunReturn,
            windows::{EventLoopBuilderExtWindows, WindowBuilderExtWindows},
        },
        window::WindowBuilder,
    };
    use wry::{NewWindowResponse, WebViewBuilder};
    let pending = Arc::new(Pending::new()?);
    let captured: Arc<Mutex<Option<Result<String, AuthError>>>> = Arc::new(Mutex::new(None));
    let mut builder = EventLoopBuilder::<()>::with_user_event();
    builder.with_any_thread(true);
    let mut events = builder.build();
    let window = WindowBuilder::new()
        .with_title("Microsoft sign-in - Andromeda Launcher")
        .with_window_classname("Andromeda Microsoft Signin WebView2")
        .with_inner_size(tao::dpi::LogicalSize::new(560.0, 740.0))
        .build(&events)
        .map_err(|_| AuthError::InvalidResponse)?;
    let state = pending.clone();
    let result = captured.clone();
    let view = WebViewBuilder::new()
        .with_url(pending.authorize_url())
        .with_incognito(true)
        .with_devtools(false)
        .with_new_window_req_handler(|_, _| NewWindowResponse::Deny)
        .with_navigation_handler(move |url| {
            let callback = state.callback(&url);
            let outcome = match callback {
                Callback::Code(code) => Some(Ok(code)),
                Callback::Error(e) => Some(Err(e)),
                Callback::Ignore => None,
            };
            if let Some(outcome) = outcome {
                if let Ok(mut slot) = result.lock() {
                    if slot.is_none() {
                        *slot = Some(outcome);
                    }
                }
                return false;
            }
            allowed_navigation(&url)
        })
        .build(&window)
        .map_err(|_| AuthError::InvalidResponse)?;
    let start = Instant::now();
    let mut code = Err(AuthError::Cancelled);
    events.run_return(|event, _, flow| {
        *flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(150));
        if let Ok(mut slot) = captured.lock() {
            if let Some(value) = slot.take() {
                code = value;
                *flow = ControlFlow::Exit;
                return;
            }
        }
        if cancel.load(Ordering::Relaxed) || start.elapsed() > Duration::from_secs(900) {
            *flow = ControlFlow::Exit;
            return;
        }
        if matches!(
            event,
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            }
        ) {
            *flow = ControlFlow::Exit;
        }
    });
    drop(view);
    drop(window);
    drop(events);
    if cancel.load(Ordering::Relaxed) {
        return Err(AuthError::Cancelled);
    }
    let code = code?;
    let pending = Arc::try_unwrap(pending).map_err(|_| AuthError::InvalidResponse)?;
    pending.exchange(&Http::new()?, &code)
}
#[cfg(not(target_os = "windows"))]
pub fn sign_in(_cancel: &AtomicBool) -> Result<Session, AuthError> {
    Err(AuthError::InvalidResponse)
}
