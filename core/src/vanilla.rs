//! Phase 1: verified vanilla downloads. Never launches Minecraft.
use reqwest::blocking::Client;
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub const VERSIONS: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const JAVA: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";
#[derive(Clone)]
pub struct Downloader {
    client: Client,
}
#[derive(Clone)]
struct Item {
    url: String,
    sha1: String,
    size: u64,
    path: PathBuf,
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .ok_or_else(|| format!("Missing string: {key}").into())
}
pub(crate) fn path(root: &Path, relative: &str) -> Result<PathBuf> {
    if relative.is_empty()
        || relative.contains(['\\', ':'])
        || Path::new(relative)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!("Unsafe manifest path: {relative}").into());
    }
    Ok(root.join(relative))
}
fn item(v: &Value, path: PathBuf) -> Result<Item> {
    let sha1 = text(v, "sha1")?.to_lowercase();
    if sha1.len() != 40 || !sha1.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid SHA-1".into());
    }
    let url = text(v, "url")?.to_owned();
    if !url.starts_with("https://") {
        return Err("Download URL must use HTTPS".into());
    }
    Ok(Item {
        url,
        sha1,
        size: v["size"].as_u64().unwrap_or(0),
        path,
    })
}
pub(crate) fn valid(p: &Path, hash: &str, size: u64) -> Result<bool> {
    if !p.is_file() {
        return Ok(false);
    }
    if size != 0 && fs::metadata(p)?.len() != size {
        return Ok(false);
    }
    let mut f = fs::File::open(p)?;
    let mut h = Sha1::new();
    let mut b = [0u8; 65536];
    loop {
        let n = f.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(format!("{:x}", h.finalize()) == hash)
}
impl Downloader {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .user_agent("AndromedaLauncher/0.0.1")
                .connect_timeout(Duration::from_secs(20))
                .timeout(Duration::from_secs(180))
                .build()?,
        })
    }
    pub fn json(&self, url: &str) -> Result<Value> {
        Ok(serde_json::from_str(&self.text(url)?)?)
    }
    pub fn text(&self, url: &str) -> Result<String> {
        if !url.starts_with("https://") {
            return Err("Catalog URL must use HTTPS".into());
        }
        let mut last = None;
        for attempt in 0..3 {
            match self
                .client
                .get(url)
                .send()
                .and_then(|r| r.error_for_status())
                .and_then(|r| r.text())
            {
                Ok(v) => return Ok(v),
                Err(e) => last = Some(e),
            }
            if attempt < 2 {
                std::thread::sleep(Duration::from_millis(500 * (attempt + 1)));
            }
        }
        Err(last.expect("three attempts").into())
    }
    fn download(&self, i: &Item) -> Result<()> {
        if valid(&i.path, &i.sha1, i.size)? {
            return Ok(());
        }
        fs::create_dir_all(i.path.parent().ok_or("Missing parent")?)?;
        let partial = i.path.with_file_name(format!(
            "{}.part",
            i.path
                .file_name()
                .ok_or("Missing filename")?
                .to_string_lossy()
        ));
        let mut last = String::new();
        for attempt in 0..3 {
            let run = (|| -> Result<()> {
                let mut response = self.client.get(&i.url).send()?.error_for_status()?;
                let mut file = fs::File::create(&partial)?;
                std::io::copy(&mut response, &mut file)?;
                file.flush()?;
                drop(file);
                if !valid(&partial, &i.sha1, i.size)? {
                    return Err("SHA-1 or size mismatch".into());
                }
                if i.path.exists() {
                    fs::remove_file(&i.path)?;
                }
                fs::rename(&partial, &i.path)?;
                Ok(())
            })();
            match run {
                Ok(()) => return Ok(()),
                Err(e) => last = e.to_string(),
            }
            let _ = fs::remove_file(&partial);
            if attempt < 2 {
                std::thread::sleep(Duration::from_millis(500 * (attempt + 1)));
            }
        }
        Err(format!("Download failed after 3 attempts: {}: {last}", i.url).into())
    }
    fn batch(&self, items: Vec<Item>) -> Result<usize> {
        let count = items.len();
        let queue = Arc::new(Mutex::new(items.into_iter()));
        let errors = Arc::new(Mutex::new(Vec::<String>::new()));
        std::thread::scope(|s| {
            for _ in 0..8 {
                let queue = queue.clone();
                let errors = errors.clone();
                s.spawn(move || loop {
                    if !errors.lock().expect("errors lock").is_empty() {
                        break;
                    }
                    let i = queue.lock().expect("queue lock").next();
                    let Some(i) = i else { break };
                    if let Err(e) = self.download(&i) {
                        errors.lock().expect("errors lock").push(e.to_string());
                        break;
                    }
                });
            }
        });
        let errors = errors.lock().map_err(|_| "Worker lock poisoned")?;
        if !errors.is_empty() {
            return Err(errors.join("\n").into());
        }
        Ok(count)
    }
    pub fn versions(&self) -> Result<Value> {
        self.json(VERSIONS)
    }
    /// Windows x64 only. All paths must be under the caller's project-owned directory.
    pub fn install(&self, root: &Path, version: &str) -> Result<Value> {
        path(root, version)?;
        if root.join("install-receipt.json").exists() {
            fs::remove_file(root.join("install-receipt.json"))?;
        }
        let manifest = self.versions()?;
        let entry = manifest["versions"]
            .as_array()
            .ok_or("Missing versions")?
            .iter()
            .find(|v| v["id"] == version)
            .ok_or("Unknown version")?;
        let metadata = path(root, &format!("versions/{version}/{version}.json"))?;
        self.download(&item(entry, metadata.clone())?)?;
        let v: Value = serde_json::from_slice(&fs::read(metadata)?)?;
        let mut items = vec![item(
            &v["downloads"]["client"],
            path(root, &format!("versions/{version}/{version}.jar"))?,
        )?];
        for lib in v["libraries"].as_array().ok_or("Missing libraries")? {
            if !allowed(lib) {
                continue;
            }
            if let Some(a) = lib["downloads"].get("artifact") {
                items.push(item(a, path(&root.join("libraries"), text(a, "path")?)?)?);
            }
            if let Some(classifier) = lib["natives"]["windows"].as_str() {
                let key = classifier.replace("${arch}", "64");
                let a = &lib["downloads"]["classifiers"][key];
                items.push(item(a, path(&root.join("libraries"), text(a, "path")?)?)?);
            }
        }
        if let Some(log) = v["logging"]["client"].get("file") {
            items.push(item(
                log,
                path(&root.join("assets/log_configs"), text(log, "id")?)?,
            )?);
        }
        println!("Downloading client and libraries");
        let libraries = self.batch(items)?;
        let ai = &v["assetIndex"];
        let index = path(
            &root.join("assets/indexes"),
            &format!("{}.json", text(ai, "id")?),
        )?;
        self.download(&item(ai, index.clone())?)?;
        let assets: Value = serde_json::from_slice(&fs::read(index)?)?;
        let mut objects = std::collections::HashMap::new();
        for obj in assets["objects"]
            .as_object()
            .ok_or("Missing asset objects")?
            .values()
        {
            let h = text(obj, "hash")?;
            if h.len() != 40 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("Invalid asset hash".into());
            }
            let value = serde_json::json!({"url":format!("https://resources.download.minecraft.net/{}/{}",&h[..2],h),"sha1":h,"size":obj["size"]});
            objects.insert(
                h.to_owned(),
                item(
                    &value,
                    path(&root.join("assets/objects"), &format!("{}/{}", &h[..2], h))?,
                )?,
            );
        }
        println!("Downloading {} verified asset objects", objects.len());
        let assets = self.batch(objects.into_values().collect())?;
        let component = text(&v["javaVersion"], "component")?;
        let runtimes = self.json(JAVA)?;
        let runtime = runtimes["windows-x64"][component]
            .as_array()
            .and_then(|a| a.first())
            .ok_or("Java runtime not available for Windows x64")?;
        let java_root = path(&root.join("runtime"), component)?;
        let runtime_manifest = java_root.join("manifest.json");
        self.download(&item(&runtime["manifest"], runtime_manifest.clone())?)?;
        let runtime: Value = serde_json::from_slice(&fs::read(runtime_manifest)?)?;
        let mut files = Vec::new();
        for (name, f) in runtime["files"]
            .as_object()
            .ok_or("Missing runtime files")?
        {
            let dest = path(&java_root, name)?;
            match text(f, "type")? {
                "directory" => fs::create_dir_all(dest)?,
                "file" => files.push(item(&f["downloads"]["raw"], dest)?),
                _ => return Err("Unsupported runtime link or entry type".into()),
            }
        }
        println!("Downloading Java runtime");
        let java_files = self.batch(files)?;
        let java = java_root.join("bin/java.exe");
        if !java.is_file() {
            return Err("Java executable missing".into());
        }
        let receipt = serde_json::json!({"version":version,"client_and_libraries":libraries,"asset_objects":assets,"java_files":java_files,"java_executable":java,"launch_tested":false,"phase":1});
        fs::create_dir_all(root)?;
        fs::write(
            root.join("install-receipt.json"),
            serde_json::to_vec_pretty(&receipt)?,
        )?;
        Ok(receipt)
    }
}
pub(crate) fn allowed(lib: &Value) -> bool {
    let Some(rules) = lib["rules"].as_array() else {
        return true;
    };
    let mut result = false;
    for rule in rules {
        let os = &rule["os"];
        let matches_os = os.is_null()
            || (os["name"].as_str().is_none_or(|n| n == "windows")
                && os["arch"].as_str().is_none_or(|a| a == "x86_64")
                && os.get("version").is_none());
        let matches_features = rule["features"]
            .as_object()
            .is_none_or(|f| f.values().all(|x| x == false));
        if matches_os && matches_features {
            result = rule["action"] == "allow";
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    fn dir() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "andromeda-download-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn rejects_unsafe_paths() {
        for p in [
            "../escape",
            "/absolute",
            "C:/escape",
            "foo\\bar",
            "a/../../b",
            "",
        ] {
            assert!(path(Path::new("safe"), p).is_err());
        }
        assert!(path(Path::new("safe"), "a/b.jar").is_ok());
    }
    #[test]
    fn verifies_hash_and_size() {
        let d = dir();
        let p = d.join("a");
        fs::write(&p, b"abc").unwrap();
        let h = "a9993e364706816aba3e25717850c26c9cd0d89d";
        assert!(valid(&p, h, 3).unwrap());
        assert!(!valid(&p, h, 4).unwrap());
        assert!(!valid(&p, &"0".repeat(40), 3).unwrap());
        fs::remove_dir_all(d).unwrap();
    }
    #[test]
    fn windows_rules() {
        assert!(allowed(&serde_json::json!({})));
        assert!(!allowed(
            &serde_json::json!({"rules":[{"action":"allow","os":{"name":"linux"}}]})
        ));
        assert!(allowed(
            &serde_json::json!({"rules":[{"action":"allow"},{"action":"disallow","os":{"name":"osx"}}]})
        ));
    }
    fn server(
        responses: Vec<(&'static str, &'static str)>,
    ) -> (String, std::thread::JoinHandle<()>) {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", l.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            for (status, body) in responses {
                let (mut s, _) = l.accept().unwrap();
                let mut b = [0; 2048];
                let _ = s.read(&mut b);
                write!(
                    s,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });
        (url, handle)
    }
    #[test]
    fn retries_http_and_corrupt_content() {
        let d = dir();
        let (url, h) = server(vec![
            ("503 Unavailable", "bad"),
            ("200 OK", "bad"),
            ("200 OK", "abc"),
        ]);
        let i = Item {
            url,
            sha1: "a9993e364706816aba3e25717850c26c9cd0d89d".into(),
            size: 3,
            path: d.join("a"),
        };
        Downloader::new().unwrap().download(&i).unwrap();
        h.join().unwrap();
        assert_eq!(fs::read(&i.path).unwrap(), b"abc");
        fs::remove_dir_all(d).unwrap();
    }
    #[test]
    fn exhaustion_never_publishes_bad_file() {
        let d = dir();
        let (url, h) = server(vec![("200 OK", "bad"); 3]);
        let i = Item {
            url,
            sha1: "a9993e364706816aba3e25717850c26c9cd0d89d".into(),
            size: 3,
            path: d.join("a"),
        };
        assert!(Downloader::new().unwrap().download(&i).is_err());
        h.join().unwrap();
        assert!(!i.path.exists());
        assert!(!d.join("a.part").exists());
        fs::remove_dir_all(d).unwrap();
    }
    #[test]
    fn valid_existing_file_skips_network() {
        let d = dir();
        let p = d.join("a");
        fs::write(&p, b"abc").unwrap();
        let i = Item {
            url: "http://127.0.0.1:1".into(),
            sha1: "a9993e364706816aba3e25717850c26c9cd0d89d".into(),
            size: 3,
            path: p,
        };
        Downloader::new().unwrap().download(&i).unwrap();
        fs::remove_dir_all(d).unwrap();
    }
}

