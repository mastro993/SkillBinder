//! Explore's catalog: fetched from a fixture source laid out like the CDN, cached with its ETag,
//! and kept through every failure. One test serves it over loopback HTTP to see the requests.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
use skillbinder_engine::{CatalogSource, Engine, EngineConfig, Logger};
use skillbinder_proto::{CatalogState, ExploreSnapshot};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tempfile::TempDir;

const HOUR: Duration = Duration::from_secs(60 * 60);

/// An isolated device whose catalog source is `<data>/catalog-source`.
struct Fixture {
    _temp: TempDir,
    data: PathBuf,
    home: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let (data, home) = (temp.path().join("data"), temp.path().join("home"));
        Self {
            _temp: temp,
            data,
            home,
        }
    }

    fn open(&self) -> Engine {
        let config = EngineConfig::isolated(self.data.clone(), self.home.clone());
        let log = Logger::open_session(&config).unwrap();
        Engine::open(config, log).unwrap()
    }
}

/// The fixture catalog the synthetic world also serves.
fn sample() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/synthetic/catalog")
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &to.join(entry.file_name()));
        } else {
            fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

/// Where an isolated engine reads its catalog.
fn source(fixture: &Fixture) -> PathBuf {
    fixture.data.join("catalog-source")
}

fn serve_sample(fixture: &Fixture) {
    copy_tree(&sample(), &source(fixture));
}

fn catalog_file(fixture: &Fixture) -> PathBuf {
    source(fixture).join("v1/catalog.json")
}

fn slugs(explore: &ExploreSnapshot) -> Vec<String> {
    explore
        .catalog
        .as_ref()
        .map(|catalog| {
            catalog
                .skills
                .iter()
                .map(|skill| skill.slug.clone())
                .collect()
        })
        .unwrap_or_default()
}

async fn refreshed(engine: &Engine, max_age: Duration) -> ExploreSnapshot {
    engine.refresh_catalog(max_age).await.unwrap();
    engine.current_explore()
}

/// Rewrites the served catalog with `edit` applied to its JSON.
fn edit_catalog(fixture: &Fixture, edit: impl FnOnce(&mut serde_json::Value)) {
    let path = catalog_file(fixture);
    let mut json: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    edit(&mut json);
    fs::write(path, json.to_string()).unwrap();
}

#[tokio::test]
async fn a_fetched_catalog_is_cached_and_served_offline_on_the_next_launch() {
    let fixture = Fixture::new();
    serve_sample(&fixture);
    let engine = fixture.open();
    assert_eq!(engine.current_explore(), ExploreSnapshot::default());
    let explore = refreshed(&engine, Duration::ZERO).await;
    assert_eq!(explore.state, CatalogState::Ready);
    let catalog = explore.catalog.clone().unwrap();
    // Ranked by installs this week, highest first.
    assert_eq!(
        slugs(&explore)[..3],
        ["ui-review", "pr-triage", "pdf-tools"]
    );
    let avatars: Vec<_> = catalog
        .creators
        .iter()
        .map(|creator| (creator.id.as_str(), creator.avatar.is_some()))
        .collect();
    assert_eq!(
        avatars,
        [
            ("northwind", true),
            ("lumen", true),
            ("harbor", true),
            ("quill", false)
        ]
    );
    let cache = fixture.data.join("catalog");
    assert!(cache.join("v1-catalog.json").is_file());
    let meta = fs::read_to_string(cache.join("v1-catalog.meta.json")).unwrap();
    assert!(meta.contains("\"etag\":\""), "{meta}");
    engine.shutdown().await.unwrap();

    fs::remove_dir_all(source(&fixture)).unwrap();
    let engine = fixture.open();
    let launched = engine.current_explore();
    assert_eq!(launched.state, CatalogState::Loading);
    assert_eq!(launched.catalog, Some(catalog.clone()));
    let offline = refreshed(&engine, Duration::ZERO).await;
    assert_eq!(offline.state, CatalogState::Offline);
    assert_eq!(offline.catalog, Some(catalog));
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn the_catalog_jobs_published_site_reads_whole() {
    // The site `xtask catalog` builds from its fixtures, so producer and consumer share one v1
    // sample.
    let fixture = Fixture::new();
    let site = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/catalog/site");
    copy_tree(&site, &source(&fixture));
    let published: serde_json::Value =
        serde_json::from_slice(&fs::read(site.join("v1/catalog.json")).unwrap()).unwrap();
    let engine = fixture.open();
    let explore = refreshed(&engine, Duration::ZERO).await;
    assert_eq!(explore.state, CatalogState::Ready);
    let catalog = explore.catalog.unwrap();
    assert_eq!(
        catalog.skills.len(),
        published["skills"].as_array().unwrap().len()
    );
    assert_eq!(
        catalog.creators.len(),
        published["creators"].as_array().unwrap().len()
    );
    for (creator, raw) in catalog
        .creators
        .iter()
        .zip(published["creators"].as_array().unwrap())
    {
        assert_eq!(
            creator.avatar.is_some(),
            raw.get("avatar").is_some(),
            "{}",
            creator.id
        );
    }
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_malformed_skill_is_dropped_and_an_oversized_or_unreadable_file_keeps_the_cache() {
    let fixture = Fixture::new();
    serve_sample(&fixture);
    edit_catalog(&fixture, |json| {
        json["skills"][1]["path"] = "../escape".into();
        json["skills"][2]["installs_week"] = "many".into();
    });
    let engine = fixture.open();
    let explore = refreshed(&engine, Duration::ZERO).await;
    assert_eq!(explore.state, CatalogState::Ready);
    assert_eq!(slugs(&explore).len(), 9);
    assert!(!slugs(&explore).contains(&"pr-triage".to_owned()));

    let file = fs::File::create(catalog_file(&fixture)).unwrap();
    file.set_len(8 * 1024 * 1024 + 1).unwrap();
    let oversized = refreshed(&engine, Duration::ZERO).await;
    assert_eq!(oversized.state, CatalogState::Unavailable);
    assert_eq!(oversized.catalog, explore.catalog);

    fs::write(catalog_file(&fixture), "<html>Gateway timeout</html>").unwrap();
    let unreadable = refreshed(&engine, Duration::ZERO).await;
    assert_eq!(unreadable.state, CatalogState::Unavailable);
    assert_eq!(unreadable.catalog, explore.catalog);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_source_without_v1_keeps_the_cache_and_needs_an_update() {
    let fixture = Fixture::new();
    serve_sample(&fixture);
    let engine = fixture.open();
    let explore = refreshed(&engine, Duration::ZERO).await;

    edit_catalog(&fixture, |json| json["schema"] = 2.into());
    let newer = refreshed(&engine, Duration::ZERO).await;
    assert_eq!(newer.state, CatalogState::Outdated);
    assert_eq!(newer.catalog, explore.catalog);

    fs::remove_dir_all(source(&fixture).join("v1")).unwrap();
    let gone = refreshed(&engine, Duration::ZERO).await;
    assert_eq!(gone.state, CatalogState::Outdated);
    assert_eq!(gone.catalog, explore.catalog);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_creator_leaving_the_catalog_takes_their_avatar() {
    let fixture = Fixture::new();
    serve_sample(&fixture);
    let engine = fixture.open();
    refreshed(&engine, Duration::ZERO).await;
    let avatar = fixture.data.join("catalog/avatars/lumen.png");
    assert!(avatar.is_file());
    edit_catalog(&fixture, |json| {
        let skills = json["skills"].as_array_mut().unwrap();
        skills.retain(|skill| skill["creator"] != "lumen");
    });
    let explore = refreshed(&engine, Duration::ZERO).await;
    let catalog = explore.catalog.unwrap();
    assert!(catalog.creators.iter().all(|creator| creator.id != "lumen"));
    assert!(!avatar.exists());
    engine.shutdown().await.unwrap();
}

/// One request the loopback server answered: its path and the headers the test reads.
#[derive(Clone, Debug)]
struct Request {
    path: String,
    if_none_match: Option<String>,
    user_agent: Option<String>,
}

/// Serves the sample over HTTP/1.1 on loopback, answering `If-None-Match` with 304 when it names
/// the file's ETag, and records every request.
fn serve_http(root: PathBuf) -> (url::Url, Arc<Mutex<Vec<Request>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = url::Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let log = requests.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let path = line.split(' ').nth(1).unwrap_or_default().to_owned();
            let mut request = Request {
                path,
                if_none_match: None,
                user_agent: None,
            };
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                let header = header.trim_end();
                if header.is_empty() {
                    break;
                }
                let (name, value) = header.split_once(": ").unwrap_or((header, ""));
                match name.to_ascii_lowercase().as_str() {
                    "if-none-match" => request.if_none_match = Some(value.to_owned()),
                    "user-agent" => request.user_agent = Some(value.to_owned()),
                    _ => {}
                }
            }
            let file = root.join(request.path.trim_start_matches('/'));
            let response = match fs::read(&file) {
                Ok(body) => {
                    let etag = format!("\"{}\"", body.len());
                    if request.if_none_match.as_deref() == Some(etag.as_str()) {
                        "HTTP/1.1 304 Not Modified\r\nConnection: close\r\n\r\n"
                            .as_bytes()
                            .to_vec()
                    } else {
                        let mut response = format!(
                            "HTTP/1.1 200 OK\r\nETag: {etag}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        )
                        .into_bytes();
                        response.extend(body);
                        response
                    }
                }
                Err(_) => {
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        .to_vec()
                }
            };
            log.lock().unwrap().push(request);
            let _ = stream.write_all(&response);
        }
    });
    (base, requests)
}

#[tokio::test]
async fn an_unchanged_catalog_costs_one_conditional_request_once_an_hour_old() {
    let fixture = Fixture::new();
    let (base, requests) = serve_http(sample());
    let mut config = EngineConfig::isolated(fixture.data.clone(), fixture.home.clone());
    config.catalog = CatalogSource::Http(base);
    let log = Logger::open_session(&config).unwrap();
    let engine = Engine::open(config, log).unwrap();
    let catalog_requests = || {
        requests
            .lock()
            .unwrap()
            .iter()
            .filter(|request| request.path == "/v1/catalog.json")
            .cloned()
            .collect::<Vec<_>>()
    };

    let explore = refreshed(&engine, Duration::ZERO).await;
    assert_eq!(explore.state, CatalogState::Ready);
    let first = catalog_requests();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].if_none_match, None);
    let agent = first[0].user_agent.clone().unwrap();
    assert!(agent.starts_with("SkillBinder/"), "{agent}");
    let avatars = requests.lock().unwrap().len() - 1;
    assert_eq!(avatars, 3);

    // Opening Explore within the hour fetches nothing.
    let fresh = refreshed(&engine, HOUR).await;
    assert_eq!(fresh, explore);
    assert_eq!(catalog_requests().len(), 1);

    // An older cache is fetched again with its ETag; the 304 keeps it.
    let confirmed = refreshed(&engine, Duration::ZERO).await;
    assert_eq!(confirmed, explore);
    let second = catalog_requests();
    assert_eq!(second.len(), 2);
    assert!(second[1].if_none_match.is_some());
    // An unchanged catalog fetches no avatar it already has.
    assert_eq!(requests.lock().unwrap().len(), 1 + avatars + 1);
    engine.shutdown().await.unwrap();
}
