#![allow(clippy::unwrap_used)]
use super::*;
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::Path,
};

/// skills.sh's answer to `GET /api/search?q=pdf&limit=20`, captured on 2026-11-19.
const LIVE: &str = include_str!("../../../../../fixtures/skills-sh/search-pdf.json");

fn results(state: SkillsShState) -> Vec<SkillsShResult> {
    match state {
        SkillsShState::Results(results) => results,
        other => panic!("no results: {other:?}"),
    }
}

/// A directory source whose `search/<query>.json` holds `body`.
fn directory(root: &Path, query: &str, body: &[u8]) -> CatalogSource {
    fs::create_dir_all(root.join("search")).unwrap();
    fs::write(root.join("search").join(format!("{query}.json")), body).unwrap();
    CatalogSource::Directory(root.to_path_buf())
}

#[test]
fn the_live_answer_translates_into_results_on_github() {
    let results = translate(LIVE.as_bytes()).unwrap();
    assert_eq!(results.len(), 20);
    assert_eq!(
        results[0],
        SkillsShResult {
            skill_id: "pdf".into(),
            key: "pdf".into(),
            name: "pdf".into(),
            url: "https://github.com/anthropics/skills.git".into(),
            installs: 233_102,
        }
    );
    // A display name is kept apart from the `skillId`.
    let ocr = results
        .iter()
        .find(|result| result.skill_id == "pdf-ocr-extraction")
        .unwrap();
    assert_eq!(ocr.name, "PDF OCR Extraction");
}

#[test]
fn unknown_fields_are_ignored_and_other_sources_and_broken_entries_dropped() {
    let payload = br#"{"skills": [
        {"id": "a/b/one", "source": "a/b", "skillId": "one", "name": "one", "installs": 5, "new": true},
        {"source": "gitlab.com/a/b", "skillId": "two", "name": "two", "installs": 1},
        {"source": "https://github.com/a/b", "skillId": "three", "name": "three", "installs": 1},
        {"source": "a/..", "skillId": "four", "name": "four", "installs": 1},
        {"source": "a/b", "skillId": " ", "name": "five", "installs": 1},
        {"source": "a/b", "skillId": "six", "installs": 1},
        {"source": "a/b", "skillId": "seven", "name": "seven", "installs": -1},
        {"source": "A/B", "skillId": "one", "name": "again", "installs": 9}
    ], "count": 8, "timings_ms": {"total": 1}}"#;
    let results = translate(payload).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].url, "https://github.com/a/b.git");
    assert_eq!(results[0].installs, 5);
}

#[test]
fn an_unreadable_or_oversized_answer_is_unavailable() {
    let temp = tempfile::tempdir().unwrap();
    for body in [&b"not json"[..], b"{\"results\": []}", b"[]"] {
        let source = directory(temp.path(), "pdf", body);
        assert_eq!(answer(&source, "pdf"), SkillsShState::Unavailable);
    }
    let mut oversized = br#"{"skills": [], "padding": ""#.to_vec();
    oversized.resize(SEARCH_LIMIT as usize + 1, b'x');
    oversized.extend(br#""}"#);
    let source = directory(temp.path(), "pdf", &oversized);
    assert_eq!(answer(&source, "pdf"), SkillsShState::Unavailable);
    // A query with no answer is a 404.
    assert_eq!(answer(&source, "docx"), SkillsShState::Unavailable);
}

#[test]
fn a_missing_source_needs_a_connection() {
    let temp = tempfile::tempdir().unwrap();
    let source = CatalogSource::Directory(temp.path().join("gone"));
    assert_eq!(answer(&source, "pdf"), SkillsShState::NeedsConnection);
}

/// A loopback server answering one request with `response`, and the request line it read.
fn serve_once(response: &'static [u8]) -> (Fetcher, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = url::Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
    let served = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 1024];
        let read = stream.read(&mut request).unwrap();
        stream.write_all(response).unwrap();
        String::from_utf8_lossy(&request[..read])
            .lines()
            .next()
            .unwrap()
            .to_owned()
    });
    (Fetcher::http(base), served)
}

#[test]
fn http_asks_with_q_and_limit_and_a_rate_limit_is_unavailable() {
    let (fetcher, served) = serve_once(
        b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
    assert_eq!(state(&fetcher, "code review"), SkillsShState::Unavailable);
    assert_eq!(
        served.join().unwrap(),
        "GET /api/search?q=code+review&limit=20 HTTP/1.1"
    );

    let (fetcher, _) = serve_once(
        b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\nConnection: close\r\n\r\n{\"skills\":[]",
    );
    assert_eq!(state(&fetcher, "pdf"), SkillsShState::Unavailable);
}

#[test]
fn a_refused_connection_needs_a_connection() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = url::Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
    drop(listener);
    assert_eq!(
        state(&Fetcher::http(base), "pdf"),
        SkillsShState::NeedsConnection
    );
}

#[tokio::test]
async fn the_last_query_wins_and_a_short_one_searches_nothing() {
    let temp = tempfile::tempdir().unwrap();
    directory(temp.path(), "pdf", LIVE.as_bytes());
    let source = directory(temp.path(), "docx", br#"{"skills": []}"#);
    let searches = Searches::new(source);

    let pdf = searches.search(" PDF ").unwrap().unwrap();
    let docx = searches.search("docx").unwrap().unwrap();
    pdf.await.unwrap();
    docx.await.unwrap();
    let search = searches.current();
    assert_eq!(search.query, "docx");
    assert_eq!(results(search.state), []);

    // Asking the shown query again sends nothing.
    assert!(searches.search("docx").unwrap().is_none());
    assert!(searches.search("p").unwrap().is_none());
    assert_eq!(searches.current().state, SkillsShState::Idle);
    searches.search("pdf").unwrap().unwrap().await.unwrap();
    assert_eq!(results(searches.current().state).len(), 20);
}
