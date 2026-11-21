//! Mod-manager UI wiring shared between the app and the headless integration
//! proof, so both run the exact same handler code. Expands per crate against
//! that crate's own generated MainWindow type.
macro_rules! wire_mods {
    ($ui:ident, $instances:ident, $instance_root:ident) => {{
        // Shared state for the mod rows and search results.
        let mod_ids = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let mod_result_ids = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        // Last selection the mod panel state belongs to, so a profile switch
        // can never inherit rows/results/actions from another profile.
        let mod_last_target = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
        // Selection the visible search results were produced under. Install
        // refuses to pair results with a different current selection.
        let mod_result_target = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));

        // Resolve the selected-in-Play instance into (game_dir, LoaderKind, mc).
        fn mod_target(
            $ui: &MainWindow,
            $instances: &std::sync::Arc<std::sync::Mutex<Vec<andromeda_core::$instances::Instance>>>,
            root: &std::path::Path,
        ) -> Option<(std::path::PathBuf, andromeda_core::catalog::LoaderKind, String)> {
            use andromeda_core::{catalog::LoaderKind, $instances::Loader};
            let inst = $ui
                .get_instance_index()
                .checked_sub(1)
                .and_then(|n| $instances.lock().ok()?.get(n as usize).cloned())?;
            let kind = match inst.loader.as_ref()? {
                Loader::Fabric { .. } => LoaderKind::Fabric,
                Loader::Quilt { .. } => LoaderKind::Quilt,
                Loader::Forge { .. } => LoaderKind::Forge,
                Loader::NeoForge { .. } => LoaderKind::NeoForge,
                Loader::LegacyFabric { .. } => LoaderKind::LegacyFabric,
                Loader::Babric { .. } => LoaderKind::Babric,
                Loader::Ornithe { .. } => LoaderKind::Ornithe,
                Loader::LiteLoader { .. } => LoaderKind::LiteLoader,
            };
            let game = andromeda_core::$instances::game_dir(root, &inst.id).ok()?;
            Some((game, kind, inst.version.clone()))
        }

        // Selection identity for change detection: the instance id behind the
        // current Play selection, if it is mod-capable.
        fn mod_target_id(
            $ui: &MainWindow,
            $instances: &std::sync::Arc<std::sync::Mutex<Vec<andromeda_core::instances::Instance>>>,
            $instance_root: &std::path::Path,
        ) -> Option<String> {
            mod_target($ui, $instances, $instance_root)?;
            let inst = $ui
                .get_instance_index()
                .checked_sub(1)
                .and_then(|n| $instances.lock().ok()?.get(n as usize).cloned())?;
            Some(inst.id)
        }

        let mr_weak = $ui.as_weak();
        let mr_instances = $instances.clone();
        let mr_root = $instance_root.clone();
        let mr_ids = mod_ids.clone();
        let mr_result_ids = mod_result_ids.clone();
        let mr_result_target = mod_result_target.clone();
        let mr_last = mod_last_target.clone();
        let mr_instances2 = mr_instances.clone();
        let mr_root2 = mr_root.clone();
        $ui.on_mods_refresh(move || {
            let Some($ui) = mr_weak.upgrade() else { return; };
            let tid = mod_target_id(&$ui, &mr_instances, &mr_root);
            let changed = {
                let mut g = match mr_last.lock() { Ok(g) => g, Err(_) => return };
                let c = *g != tid;
                *g = tid.clone();
                c
            };
            if changed {
                // Selection moved: drop the previous profile's search state so
                // it can never be shown or installed against this one.
                if let Ok(mut v) = mr_result_ids.lock() { v.clear(); }
                if let Ok(mut v) = mr_result_target.lock() { *v = None; }
                $ui.set_mod_result_index(0);
                $ui.set_mod_results(slint::ModelRc::new(slint::VecModel::from(Vec::<slint::SharedString>::new())));
                $ui.set_mod_result_details(slint::ModelRc::new(slint::VecModel::from(Vec::<slint::SharedString>::new())));
            }
            let supported = tid.is_some();
            $ui.set_mod_supported(supported);
            if !supported {
                $ui.set_mod_rows(slint::ModelRc::new(slint::VecModel::from(Vec::<slint::SharedString>::new())));
                $ui.set_mod_row_details(slint::ModelRc::new(slint::VecModel::from(Vec::<slint::SharedString>::new())));
                if let Ok(mut v) = mr_ids.lock() { v.clear(); }
                return;
            }
            let Some((game, _, _)) = mod_target(&$ui, &mr_instances, &mr_root) else { return; };
            let weak = mr_weak.clone();
            let ids = mr_ids.clone();
            let tid_load = tid.clone();
            let i2 = mr_instances2.clone();
            let r2 = mr_root2.clone();
            std::thread::spawn(move || {
                let result = (|| -> Result<(Vec<String>, Vec<slint::SharedString>, Vec<slint::SharedString>), Box<dyn std::error::Error + Send + Sync>> {
                    let reg = andromeda_core::mods::load(&game)?;
                    let mut pids = Vec::new();
                    let mut labels = Vec::new();
                    let mut details = Vec::new();
                    for m in &reg.mods {
                        pids.push(m.project_id.clone());
                        labels.push(m.title.clone().into());
                        let state = if m.enabled { "on" } else { "off" };
                        let role = if m.required_by.is_some() { " · dependency" } else { "" };
                        details.push(format!("{}{state}{role}", m.version_number).into());
                    }
                    Ok((pids, labels, details))
                })();
                let _ = weak.upgrade_in_event_loop(move |$ui| {
                    if mod_target_id(&$ui, &i2, &r2) != tid_load {
                        // Selection moved while the registry loaded: never show
                        // another profile's rows here.
                        return;
                    }
                    match result {
                    Ok((pids, labels, details)) => {
                        if let Ok(mut v) = ids.lock() { *v = pids; }
                        $ui.set_mod_index(0);
                        $ui.set_mod_rows(slint::ModelRc::new(slint::VecModel::from(labels)));
                        $ui.set_mod_row_details(slint::ModelRc::new(slint::VecModel::from(details)));
                    }
                    Err(e) => $ui.set_mod_status(format!("Could not load mods: {e}").into()),
                    }
                });
            });
        });

        // Shared body for toggle / remove / update / install: one selected project,
        // one blocking core call, then a refresh.
        macro_rules! mod_action {
            ($on:ident, $body:expr) => {{
                let weak = $ui.as_weak();
                let insts = $instances.clone();
                let root = $instance_root.clone();
                let ids = mod_ids.clone();
                $ui.$on(move || {
                    let Some($ui) = weak.upgrade() else { return; };
                    if $ui.get_launch_busy() || $ui.get_auth_busy() { return; }
                    let Some((game, kind, mc)) = mod_target(&$ui, &insts, &root) else { return; };
                    let tid0 = mod_target_id(&$ui, &insts, &root);
                    let pid = ids.lock().ok().and_then(|v| v.get($ui.get_mod_index() as usize).cloned());
                    let cache = root.join("catalog-cache");
                    $ui.set_launch_busy(true);
                    let w = weak.clone();
                    let run = $body;
                    let insts2 = insts.clone();
                    let root2 = root.clone();
                    std::thread::spawn(move || {
                        let msg = run(pid, game, kind, mc, cache);
                        let _ = w.upgrade_in_event_loop(move |$ui| {
                            // Disk always got the profile captured at invoke; if
                            // the selection moved meanwhile, don't post that
                            // profile's status onto the new one.
                            if mod_target_id(&$ui, &insts2, &root2) == tid0 {
                                match msg {
                                    Ok(text) => $ui.set_mod_status(text.into()),
                                    Err(e) => $ui.set_mod_status(format!("{e}").into()),
                                }
                            }
                            $ui.set_launch_busy(false);
                            $ui.invoke_mods_refresh();
                        });
                    });
                });
            }};
        }

        type ModResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

        mod_action!(on_mod_toggle, |pid: Option<String>, game: std::path::PathBuf, _kind: andromeda_core::catalog::LoaderKind, _mc: String, _cache: std::path::PathBuf| -> ModResult<String> {
            let pid = pid.ok_or("No mod selected")?;
            let mut reg = andromeda_core::mods::load(&game)?;
            let now = !reg.get(&pid).ok_or("Mod no longer managed")?.enabled;
            andromeda_core::mods::set_enabled(&game, &mut reg, &pid, now)?;
            Ok(format!("Mod switched {}.", if now { "on" } else { "off" }))
        });

        mod_action!(on_mod_remove, |pid: Option<String>, game: std::path::PathBuf, _kind: andromeda_core::catalog::LoaderKind, _mc: String, _cache: std::path::PathBuf| -> ModResult<String> {
            let pid = pid.ok_or("No mod selected")?;
            let mut reg = andromeda_core::mods::load(&game)?;
            let orphans = andromeda_core::mods::orphans(&reg, &pid);
            let gone = andromeda_core::mods::remove(&game, &mut reg, &pid)?;
            if orphans.is_empty() {
                Ok(format!("Removed {}.", gone.title))
            } else {
                Ok(format!("Removed {}. No longer needed: {}.", gone.title, orphans.join(", ")))
            }
        });

        mod_action!(on_mod_update, |pid: Option<String>, game: std::path::PathBuf, kind: andromeda_core::catalog::LoaderKind, mc: String, cache: std::path::PathBuf| -> ModResult<String> {
            let pid = pid.ok_or("No mod selected")?;
            let dl = andromeda_core::vanilla::Downloader::new()?;
            let out = andromeda_core::modrinth_install::install_project(&dl, &cache, &game, kind, &mc, &pid, None)?;
            Ok(if out.installed.is_empty() { "Already up to date.".into() } else { format!("Updated ({} step(s)).", out.installed.len()) })
        });

        let ms_weak = $ui.as_weak();
        let ms_instances = $instances.clone();
        let ms_root = $instance_root.clone();
        let ms_result_ids = mod_result_ids.clone();
        let ms_result_target = mod_result_target.clone();
        $ui.on_mods_search(move || {
            let Some($ui) = ms_weak.upgrade() else { return; };
            if $ui.get_launch_busy() { return; }
            let Some((_, kind, _)) = mod_target(&$ui, &ms_instances, &ms_root) else { return; };
            let tid = mod_target_id(&$ui, &ms_instances, &ms_root);
            let query = $ui.get_mod_query().to_string();
            if query.trim().is_empty() { return; }
            $ui.set_launch_busy(true);
            let weak = ms_weak.clone();
            let root = ms_root.clone();
            let ids = ms_result_ids.clone();
            let ms_instances2 = ms_instances.clone();
            let ms_root2 = ms_root.clone();
            let ids_target = ms_result_target.clone();
            std::thread::spawn(move || {
                let result = (|| -> ModResult<(Vec<String>, Vec<slint::SharedString>, Vec<slint::SharedString>)> {
                    let dl = andromeda_core::vanilla::Downloader::new()?;
                    let cache = root.join("catalog-cache");
                    let (hits, stale) = andromeda_core::modrinth::search(&dl, &cache, andromeda_core::modrinth::facet(kind), &query)?;
                    let mut pids = Vec::new();
                    let mut labels = Vec::new();
                    let mut details = Vec::new();
                    for h in hits {
                        pids.push(h.id.clone());
                        labels.push(h.title.into());
                        details.push(format!("{} downloads{}", h.downloads, if stale { " · cached" } else { "" }).into());
                    }
                    Ok((pids, labels, details))
                })();
                let _ = weak.upgrade_in_event_loop(move |$ui| {
                    if mod_target_id(&$ui, &ms_instances2, &ms_root2) != tid {
                        // Profile switched mid-search: discard these results.
                        $ui.set_launch_busy(false);
                        return;
                    }
                    match result {
                        Ok((pids, labels, details)) => {
                            if let Ok(mut g) = ids_target.lock() { *g = tid.clone(); }
                            if let Ok(mut v) = ids.lock() { *v = pids; }
                            $ui.set_mod_result_index(0);
                            $ui.set_mod_results(slint::ModelRc::new(slint::VecModel::from(labels)));
                            $ui.set_mod_result_details(slint::ModelRc::new(slint::VecModel::from(details)));
                        }
                        Err(e) => $ui.set_mod_status(format!("Search failed: {e}").into()),
                    }
                    $ui.set_launch_busy(false);
                });
            });
        });

        let mi_weak = $ui.as_weak();
        let mi_instances = $instances.clone();
        let mi_root = $instance_root.clone();
        let mi_result_ids = mod_result_ids.clone();
        let mi_result_target = mod_result_target.clone();
        $ui.on_mod_install(move || {
            let Some($ui) = mi_weak.upgrade() else { return; };
            if $ui.get_launch_busy() || $ui.get_auth_busy() { return; }
            let Some((game, kind, mc)) = mod_target(&$ui, &mi_instances, &mi_root) else { return; };
            let tid0 = mod_target_id(&$ui, &mi_instances, &mi_root);
            // The visible results belong to the selection that produced them:
            // never pair them with a different current selection.
            let origin = mi_result_target.lock().ok().and_then(|g| g.clone());
            if origin.is_none() || origin != tid0 {
                $ui.set_mod_status("Those results belong to another profile. Search again here.".into());
                return;
            }
            let pid = match mi_result_ids.lock().ok().and_then(|v| v.get($ui.get_mod_result_index() as usize).cloned()) {
                Some(p) => p,
                None => return,
            };
            $ui.set_launch_busy(true);
            let weak = mi_weak.clone();
            let root = mi_root.clone();
            let mi_instances2 = mi_instances.clone();
            let mi_root2 = mi_root.clone();
            std::thread::spawn(move || {
                let result = (|| -> ModResult<String> {
                    let dl = andromeda_core::vanilla::Downloader::new()?;
                    let cache = root.join("catalog-cache");
                    let out = andromeda_core::modrinth_install::install_project(&dl, &cache, &game, kind, &mc, &pid, None)?;
                    let mut parts = vec![format!("Installed ({} step(s)).", out.installed.len())];
                    if !out.unstable.is_empty() {
                        parts.push(format!("Beta/alpha build for: {}.", out.unstable.join(", ")));
                    }
                    if !out.optional.is_empty() {
                        let names: Vec<String> = out.optional.iter().map(|(p, _)| p.clone()).collect();
                        parts.push(format!("Optional extras: {}.", names.join(", ")));
                    }
                    Ok(parts.join(" "))
                })();
                let _ = weak.upgrade_in_event_loop(move |$ui| {
                    // Same rule as mod_action: disk got the invoked profile,
                    // status only lands if the selection is still that profile.
                    if mod_target_id(&$ui, &mi_instances2, &mi_root2) == tid0 {
                        match result {
                            Ok(text) => $ui.set_mod_status(text.into()),
                            Err(e) => $ui.set_mod_status(format!("{e}").into()),
                        }
                    }
                    $ui.set_launch_busy(false);
                    $ui.invoke_mods_refresh();
                });
            });
        });
    }};
}
pub(crate) use wire_mods;
