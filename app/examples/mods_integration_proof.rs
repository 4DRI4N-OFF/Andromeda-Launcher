#![allow(deprecated)] // slint inline-import emits a deprecated fallback const in this proof target; no behavior change
//! Integration proof for the mod manager: the REAL UI handlers (the same
//! wire_mods! expansion the app runs) against a TEMP instance on disk and the
//! LIVE Modrinth API. Distinguishes "buttons dispatch" (mods_section_proof)
//! from the functional flow:
//!   refresh -> search "sodium" -> install first result -> toggle off -> remove
//! asserting on real disk state (jars, registry) and UI state at each step.
//! Run: cargo run --example mods_integration_proof
use slint::{
    platform::{
        software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
        Platform, WindowAdapter,
    },
    ComponentHandle, Model, ModelRc, SharedString, VecModel,
};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
slint::slint! { import { MainWindow } from "../ui/main.slint"; }
#[allow(dead_code)]
#[path = "../src/mods_wire.rs"]
mod mods_wire;
// Minimal headless platform WITH an event loop: the real handlers post back
// through upgrade_in_event_loop, so a plain MinimalSoftwareWindow pump is not
// enough; queued closures run on this thread via the proxy channel.
struct Offscreen {
    win: Rc<MinimalSoftwareWindow>,
    tx: mpsc::Sender<Box<dyn FnOnce() + Send>>,
    rx: Mutex<mpsc::Receiver<Box<dyn FnOnce() + Send>>>,
    quitting: Arc<AtomicBool>,
}
struct Proxy {
    tx: mpsc::Sender<Box<dyn FnOnce() + Send>>,
    quitting: Arc<AtomicBool>,
}
impl slint::platform::EventLoopProxy for Proxy {
    fn quit_event_loop(&self) -> Result<(), slint::EventLoopError> {
        self.quitting.store(true, Ordering::SeqCst);
        let _ = self.tx.send(Box::new(|| {}));
        Ok(())
    }
    fn invoke_from_event_loop(
        &self,
        event: Box<dyn FnOnce() + Send>,
    ) -> Result<(), slint::EventLoopError> {
        self.tx
            .send(event)
            .map_err(|_| slint::EventLoopError::EventLoopTerminated)
    }
}
impl Platform for Offscreen {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.win.clone())
    }
    fn new_event_loop_proxy(&self) -> Option<Box<dyn slint::platform::EventLoopProxy>> {
        Some(Box::new(Proxy {
            tx: self.tx.clone(),
            quitting: self.quitting.clone(),
        }))
    }
    fn run_event_loop(&self) -> Result<(), slint::PlatformError> {
        loop {
            if self.quitting.load(Ordering::SeqCst) {
                return Ok(());
            }
            let next = {
                let rx = self.rx.lock().unwrap();
                rx.recv_timeout(Duration::from_millis(10)).ok()
            };
            if let Some(f) = next {
                f();
            }
            slint::platform::update_timers_and_animations();
            self.win.request_redraw();
            self.win.draw_if_needed(|_| {});
        }
    }
}
// Run f on the event loop and return its value to the caller thread.
fn ui_read<T: Send + 'static>(
    weak: &slint::Weak<MainWindow>,
    f: impl Fn(&MainWindow) -> T + Send + 'static,
) -> T {
    let (tx, rx) = mpsc::channel();
    let w = weak.clone();
    slint::invoke_from_event_loop(move || {
        if let Some(u) = w.upgrade() {
            let _ = tx.send(f(&u));
        }
    })
    .unwrap();
    rx.recv().unwrap()
}
fn poll<T: Send + PartialEq + std::fmt::Debug + 'static>(
    weak: &slint::Weak<MainWindow>,
    what: &str,
    secs: u64,
    want: T,
    f: impl Fn(&MainWindow) -> T + Send + Copy + 'static,
) {
    let t0 = Instant::now();
    loop {
        let v = ui_read(weak, f);
        if v == want {
            return;
        }
        assert!(
            t0.elapsed() < Duration::from_secs(secs),
            "timeout waiting for {what}: last={v:?} want={want:?}"
        );
        std::thread::sleep(Duration::from_millis(500));
    }
}
fn main() -> Result<(), slint::PlatformError> {
    let (qtx, qrx) = mpsc::channel::<Box<dyn FnOnce() + Send>>();
    let quitting = Arc::new(AtomicBool::new(false));
    let w = Offscreen {
        win: MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer),
        tx: qtx,
        rx: Mutex::new(qrx),
        quitting,
    };
    slint::platform::set_platform(Box::new(w)).unwrap();
    let ui = MainWindow::new()?;
    ui.set_reduce_animations(true);
    // One real Fabric instance on disk in a throwaway root.
    let root = std::env::temp_dir().join(format!("andromeda-mods-it-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    // Two loader instances: "Another" (Quilt) sorts before "Proof" (Fabric).
    andromeda_core::instances::create_with_loader(
        &root,
        "Another",
        "player",
        2048,
        Some(andromeda_core::instances::Loader::Quilt {
            version: "0.28.1".into(),
        }),
    )
    .unwrap();
    let inst = andromeda_core::instances::create_with_loader(
        &root,
        "Proof",
        "player",
        2048,
        Some(andromeda_core::instances::Loader::Fabric {
            version: "0.16.10".into(),
        }),
    )
    .unwrap();
    let instances = Arc::new(Mutex::new(andromeda_core::instances::list(&root).unwrap()));
    ui.set_instance_names(ModelRc::new(VecModel::from(vec![
        SharedString::from("Default profile (phase 2)"),
        SharedString::from("Another"),
        SharedString::from("Proof"),
    ])));
    // UI index: 1 = Another (instances[0]), 2 = Proof (instances[1]).
    ui.set_instance_index(2);
    let instance_root = root.clone();
    mods_wire::wire_mods!(ui, instances, instance_root);
    let game = root.join(&inst.id).join("game");
    let weak = ui.as_weak();
    let g = game.clone();
    let root2 = root.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        // 1. Refresh: the real handler must see the Fabric instance as mod-capable.
        ui_read(&weak, |u| u.invoke_mods_refresh());
        poll(&weak, "mod-supported", 30, true, |u| u.get_mod_supported());
        println!("integration: refresh marked the Fabric instance mod-supported");
        // 2a. Profile switch MID-SEARCH: the late results must be discarded, never
        // shown or installable against the newly selected profile.
        ui_read(&weak, |u| u.set_mod_query(SharedString::from("sodium")));
        ui_read(&weak, |u| u.invoke_mods_search());
        ui_read(&weak, |u| u.set_instance_index(1)); // Another (Quilt)
        ui_read(&weak, |u| u.invoke_mods_refresh());
        poll(&weak, "mid-switch busy", 120, false, |u| {
            u.get_launch_busy()
        });
        let n = ui_read(&weak, |u| u.get_mod_results().row_count());
        assert_eq!(n,0,"search results from the previous profile must be discarded after a mid-search switch, got {n}");
        assert!(
            ui_read(&weak, |u| u.get_mod_supported()),
            "Another (Quilt) must be mod-capable"
        );
        println!("integration: mid-search profile switch discarded the late results");
        // 2b. Back to Proof; a fresh search now belongs to this selection.
        ui_read(&weak, |u| u.set_instance_index(2));
        ui_read(&weak, |u| u.invoke_mods_refresh());
        ui_read(&weak, |u| u.set_mod_query(SharedString::from("sodium")));
        ui_read(&weak, |u| u.invoke_mods_search());
        let t0 = Instant::now();
        loop {
            let (n, st, busy) = ui_read(&weak, |u| {
                (
                    u.get_mod_results().row_count(),
                    u.get_mod_status().to_string(),
                    u.get_launch_busy(),
                )
            });
            if n > 0 {
                println!("integration: live search returned {n} result(s)");
                break;
            }
            if t0.elapsed().as_secs() % 10 < 1 {
                eprintln!("waiting search: results={n} busy={busy} status={st:?}");
            }
            assert!(
                t0.elapsed() < Duration::from_secs(90),
                "timeout waiting for live Modrinth search (status={st:?} busy={busy})"
            );
            std::thread::sleep(Duration::from_millis(500));
        }
        // 2c. Origin identity: with Proof's results still visible, move the selection
        // to Another WITHOUT a refresh and press Install: it must refuse and install
        // nothing anywhere. Back on Proof the same results stay valid (same origin).
        let another_game = root2.join("another").join("game");
        std::fs::create_dir_all(&another_game).unwrap(); // launch would create it; the refusal test reads its registry
        ui_read(&weak, |u| u.set_instance_index(1));
        ui_read(&weak, |u| u.invoke_mod_install());
        let st = ui_read(&weak, |u| u.get_mod_status().to_string());
        assert!(
            st.contains("another profile"),
            "stale results must refuse install on a different selection, got: {st}"
        );
        let reg_a = andromeda_core::mods::load(&another_game).unwrap();
        assert!(
            reg_a.mods.is_empty(),
            "refused install must not touch Another's profile"
        );
        let reg_p = andromeda_core::mods::load(&g).unwrap();
        assert!(
            reg_p.mods.is_empty(),
            "refused install must not touch Proof's profile"
        );
        println!("integration: install with stale results refused cross-profile, nothing written");
        ui_read(&weak, |u| u.set_instance_index(2));
        ui_read(&weak, |u| u.invoke_mods_refresh());
        poll(&weak, "results kept after no-op detour", 30, 24, |u| {
            u.get_mod_results().row_count()
        });
        // 3. Install the first result: jars + registry on disk, rows refreshed in UI.
        ui_read(&weak, |u| u.set_mod_status(SharedString::from("")));
        ui_read(&weak, |u| u.set_mod_result_index(0));
        ui_read(&weak, |u| u.invoke_mod_install());
        // Switch the selection MID-INSTALL: disk must still get Proof, but the late
        // status must not be posted onto Another, and Another's rows must show.
        ui_read(&weak, |u| u.set_instance_index(1));
        ui_read(&weak, |u| u.invoke_mods_refresh());
        poll(&weak, "install busy", 240, false, |u| u.get_launch_busy());
        let reg = andromeda_core::mods::load(&g).expect("registry must load after install");
        assert_eq!(
            reg.mods.len(),
            1,
            "exactly one managed mod after install: {:?}",
            reg.mods.iter().map(|m| &m.title).collect::<Vec<_>>()
        );
        let entry = reg.mods[0].clone();
        let jar = g.join("mods").join(entry.disk_name());
        assert!(
            jar.exists(),
            "installed jar must exist at {}",
            jar.display()
        );
        let reg_a = andromeda_core::mods::load(&another_game).unwrap();
        assert!(
            reg_a.mods.is_empty(),
            "mid-install switch must not leak into Another's profile"
        );
        let status = ui_read(&weak, |u| u.get_mod_status().to_string());
        assert!(
            status.is_empty(),
            "late install status must not be posted onto Another, got: {status}"
        );
        poll(&weak, "Another rows", 30, 0, |u| {
            u.get_mod_rows().row_count()
        });
        println!("integration: mid-install switch - disk got Proof ({}), no status/rows leaked to Another",entry.disk_name());
        // Back to Proof: its row appears on refresh.
        ui_read(&weak, |u| u.set_instance_index(2));
        ui_read(&weak, |u| u.invoke_mods_refresh());
        poll(&weak, "mod rows", 30, 1, |u| u.get_mod_rows().row_count());
        println!(
            "integration: installed '{}' ({}), jar on disk, UI row populated",
            entry.title, entry.version_number
        );
        // 3b. Update path, no-op first: latest already installed.
        ui_read(&weak, |u| u.set_mod_index(0));
        ui_read(&weak, |u| u.invoke_mod_update());
        poll(&weak, "update-noop busy", 120, false, |u| {
            u.get_launch_busy()
        });
        let st = ui_read(&weak, |u| u.get_mod_status().to_string());
        assert!(
            st.contains("up to date"),
            "no-op update must say so, got: {st}"
        );
        let reg = andromeda_core::mods::load(&g).unwrap();
        assert_eq!(
            reg.mods[0].version_number, entry.version_number,
            "no-op update must not change the version"
        );
        println!("integration: update no-op path reports already-up-to-date");
        // 3c. Real update: downgrade on disk to the oldest compatible build via the
        // core (UI always installs latest), then Update must move it back to latest.
        let cache = root2.join("catalog-cache");
        let dl = andromeda_core::vanilla::Downloader::new().unwrap();
        // install_project prefers stable builds (betas are reported, not picked), so
        // the update target is the newest STABLE. Sodium has a single stable for
        // 1.21.11; walk known projects until one has two stables to move between.
        let mut picked = None;
        for pid in ["AANobbMI", "gvQqBUqZ", "mOgUt4GM", "P7dR8mSH"] {
            let (versions, _) =
                andromeda_core::modrinth::versions(&dl, &cache, pid, "fabric", "1.21.11").unwrap();
            let stables: Vec<_> = versions
                .iter()
                .filter(|v| matches!(v.release, andromeda_core::modrinth::Release::Release))
                .collect();
            if stables.len() >= 2 {
                picked = Some((
                    pid.to_string(),
                    stables[0].clone(),
                    stables[stables.len() - 1].clone(),
                ));
                break;
            }
        }
        let Some((upid, latestv, oldv)) = picked else {
            panic!("no candidate project with two stable builds for 1.21.11/fabric")
        };
        println!(
            "integration: update path uses project {upid} ({} -> {})",
            oldv.number, latestv.number
        );
        // install_project no-ops over a newer install, so downgrade via a clean
        // state: remove through the UI, then core-install the pinned older build.
        ui_read(&weak, |u| u.invoke_mod_remove());
        poll(&weak, "pre-downgrade remove busy", 60, false, |u| {
            u.get_launch_busy()
        });
        {
            let reg = andromeda_core::mods::load(&g).unwrap();
            assert!(
                reg.mods.is_empty(),
                "remove must clear the registry before the downgrade"
            );
        }
        let out = andromeda_core::modrinth_install::install_project(
            &dl,
            &cache,
            &g,
            andromeda_core::catalog::LoaderKind::Fabric,
            "1.21.11",
            &upid,
            Some(&oldv.id),
        )
        .unwrap();
        assert!(
            !out.installed.is_empty(),
            "downgrade to {} must install",
            oldv.number
        );
        let reg = andromeda_core::mods::load(&g).unwrap();
        assert_eq!(
            reg.mods[0].version_number, oldv.number,
            "registry must show the downgraded build"
        );
        let before_name = reg.mods[0].disk_name();
        let before_size = std::fs::metadata(g.join("mods").join(&before_name))
            .unwrap()
            .len();
        ui_read(&weak, |u| u.invoke_mods_refresh());
        poll(&weak, "rows after downgrade", 30, 1, |u| {
            u.get_mod_rows().row_count()
        });
        ui_read(&weak, |u| u.set_mod_index(0));
        ui_read(&weak, |u| u.invoke_mod_update());
        poll(&weak, "update busy", 240, false, |u| u.get_launch_busy());
        let reg = andromeda_core::mods::load(&g).unwrap();
        let got = &reg.mods[0];
        assert_eq!(
            got.version_number, latestv.number,
            "update must move {} to latest stable {}",
            got.version_number, latestv.number
        );
        assert_ne!(
            got.version_number, oldv.number,
            "update must leave the downgraded build"
        );
        assert!(
            g.join("mods").join(got.disk_name()).exists(),
            "updated jar must exist on disk"
        );
        let after_size = std::fs::metadata(g.join("mods").join(got.disk_name()))
            .unwrap()
            .len();
        assert_ne!(
            before_name,
            got.disk_name(),
            "update must replace the jar file"
        );
        println!("integration: update evidence before={} ({} bytes) after={} ({} bytes); install_project verifies the Modrinth sha512 of every download",before_name,before_size,got.disk_name(),after_size);
        let st = ui_read(&weak, |u| u.get_mod_status().to_string());
        assert!(
            st.contains("Updated"),
            "update must report steps, got: {st}"
        );
        println!(
            "integration: update moved {} -> {} on disk and in the registry",
            oldv.number, got.version_number
        );
        // 4. Toggle off: registry + rename on disk via the real handler.
        ui_read(&weak, |u| u.set_mod_index(0));
        ui_read(&weak, |u| u.invoke_mod_toggle());
        poll(&weak, "toggle busy", 60, false, |u| u.get_launch_busy());
        let reg = andromeda_core::mods::load(&g).unwrap();
        assert!(
            !reg.get(&entry.project_id).unwrap().enabled,
            "mod must be disabled after toggle"
        );
        println!("integration: toggle off persisted to registry");
        // 5. Remove: registry empty, jar gone.
        ui_read(&weak, |u| u.invoke_mod_remove());
        poll(&weak, "remove busy", 60, false, |u| u.get_launch_busy());
        let reg = andromeda_core::mods::load(&g).unwrap();
        assert!(
            reg.get(&entry.project_id).is_none(),
            "mod must be gone from registry after remove"
        );
        assert!(
            !g.join("mods").join(entry.disk_name()).exists(),
            "jar must be removed from disk"
        );
        println!("integration: remove cleared registry and disk");
        println!("Integration proof complete: real handlers + temp filesystem + live Modrinth, all assertions green.");
        let _ = slint::quit_event_loop();
    });
    let r = slint::run_event_loop();
    let _ = std::fs::remove_dir_all(&root);
    r
}
