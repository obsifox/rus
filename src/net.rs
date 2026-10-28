//! HTTP helpers and the resumable part downloader.
//!
//! Large files are split into fixed parts (`dest.parts/`). Each part is fetched
//! with an HTTP Range request and kept on disk once its length checks out. If a
//! part is short, corrupt, or the connection drops, only that part is retried —
//! completed parts are never downloaded again, including across a process
//! restart. After the parts are joined, the whole-file hash is checked; a
//! mismatch re-fetches the parts that no longer match and continues until the
//! file is good or every part has been replaced.
use sha1::{Digest, Sha1};
use sha2::Sha256;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

const PART_SIZE: u64 = 2 * 1024 * 1024;
const PART_TRIES: u32 = 12;

pub fn agent() -> &'static ureq::Agent {
    static A: OnceLock<ureq::Agent> = OnceLock::new();
    A.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(20))
            .timeout_read(Duration::from_secs(180))
            .user_agent("Rus/1.0 (Minecraft launcher)")
            .build()
    })
}

/// Agent that does not follow redirects, so a Range header can be re-applied
/// on the GitHub / CDN hop instead of being dropped.
fn direct_agent() -> &'static ureq::Agent {
    static A: OnceLock<ureq::Agent> = OnceLock::new();
    A.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(20))
            .timeout_read(Duration::from_secs(180))
            .redirects(0)
            .user_agent("Rus/1.0 (Minecraft launcher)")
            .build()
    })
}

pub fn get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let mut last = String::new();
    for attempt in 0..4u32 {
        match call(url, None) {
            Ok((_, resp)) => {
                let mut buf = Vec::new();
                resp.into_reader()
                    .read_to_end(&mut buf)
                    .map_err(|e| format!("{url}: {e}"))?;
                return Ok(buf);
            }
            Err(e) => {
                last = e;
                backoff(attempt);
            }
        }
    }
    Err(last)
}

pub fn get_json(url: &str) -> Result<serde_json::Value, String> {
    let bytes = get_bytes(url)?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{url}: JSON {e}"))
}

/// Stream `url` to `dest`, reporting 0..1 as bytes arrive. `cancel` stops the
/// next chunk and deletes the partial file. Completed parts of a version
/// install still use `download`; this path is the visible Explore bar.
pub fn copy_url(url: &str, dest: &Path, cancel: &AtomicBool, mut on: impl FnMut(f32)) -> Result<u64, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("canceled".into());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let leaf = dest.file_name().and_then(|s| s.to_str()).unwrap_or("download");
    let tmp = dest.with_file_name(format!(".{leaf}.part"));
    let (_, resp) = call(url, None)?;
    let total = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let mut file = File::create(&tmp).map_err(|e| e.to_string())?;
    let mut reader = resp.into_reader();
    let mut buf = vec![0u8; 64 * 1024];
    let mut got = 0u64;
    let result = (|| -> Result<(), String> {
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("canceled".into());
            }
            let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            got += n as u64;
            if total > 0 {
                on((got as f32 / total as f32).clamp(0.0, 0.99));
            } else if got > 0 {
                on((got as f32 / (got as f32 + 512.0 * 1024.0)).min(0.95));
            }
        }
        file.flush().map_err(|e| e.to_string())?;
        Ok(())
    })();
    drop(file);
    if let Err(e) = result {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    if got == 0 {
        let _ = fs::remove_file(&tmp);
        return Err("empty download".into());
    }
    if let Err(e) = fs::rename(&tmp, dest) {
        let _ = fs::remove_file(&tmp);
        return Err(e.to_string());
    }
    on(1.0);
    Ok(got)
}

pub fn sha1_hex(bytes: &[u8]) -> String {
    let mut h = Sha1::new();
    h.update(bytes);
    hex(&h.finalize())
}

pub fn sha256_file(path: &Path) -> Option<String> {
    hash_file(path, true)
}

pub fn sha1_file(path: &Path) -> Option<String> {
    hash_file(path, false)
}

fn hash_file(path: &Path, sha256: bool) -> Option<String> {
    let mut f = File::open(path).ok()?;
    let mut buf = [0u8; 64 * 1024];
    if sha256 {
        let mut h = Sha256::new();
        loop {
            let n = f.read(&mut buf).ok()?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
        }
        Some(hex(&h.finalize()))
    } else {
        let mut h = Sha1::new();
        loop {
            let n = f.read(&mut buf).ok()?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
        }
        Some(hex(&h.finalize()))
    }
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// True when the file already exists and matches the expected hash (if one is known).
pub fn verify(path: &Path, expected: Option<&str>) -> bool {
    if !path.is_file() {
        return false;
    }
    hash_ok(path, expected)
}

fn hash_ok(path: &Path, expected: Option<&str>) -> bool {
    let Some(exp) = expected.map(str::trim).filter(|s| !s.is_empty()) else {
        return path.is_file();
    };
    let got = if exp.len() == 64 {
        sha256_file(path)
    } else {
        sha1_file(path)
    };
    got.map(|h| h.eq_ignore_ascii_case(exp)).unwrap_or(false)
}

pub fn download(
    url: &str,
    dest: &Path,
    expected: Option<&str>,
    on_bytes: impl FnMut(u64),
) -> Result<u64, String> {
    download_with(url, dest, expected, on_bytes)
}

pub fn download_with(
    url: &str,
    dest: &Path,
    expected: Option<&str>,
    on_bytes: impl FnMut(u64),
) -> Result<u64, String> {
    download_inner(url, dest, expected, PART_SIZE, on_bytes)
}

fn download_inner(
    url: &str,
    dest: &Path,
    expected: Option<&str>,
    part_size: u64,
    mut on_bytes: impl FnMut(u64),
) -> Result<u64, String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if verify(dest, expected) {
        return Ok(dest.metadata().map(|m| m.len()).unwrap_or(0));
    }
    let part_size = part_size.max(1);
    if let Some(man) = load_manifest(dest) {
        if man.url == url && man.part_size == part_size && man.total > part_size {
            return finish_parts(url, dest, expected, man, &mut on_bytes);
        }
    }
    match probe(url) {
        Ok((total, ranged)) if ranged && total > part_size => {
            let man = Manifest {
                url: url.to_string(),
                total,
                part_size,
                done: Vec::new(),
            };
            save_manifest(dest, &man)?;
            finish_parts(url, dest, expected, man, &mut on_bytes)
        }
        Ok((total, _)) if total > 0 => download_stream(url, dest, expected, Some(total), &mut on_bytes),
        _ => download_stream(url, dest, expected, None, &mut on_bytes),
    }
}

struct Manifest {
    url: String,
    total: u64,
    part_size: u64,
    /// index → crc of a completed part. Missing index means not done.
    done: Vec<Option<u32>>,
}

fn parts_dir(dest: &Path) -> PathBuf {
    sibling(dest, ".parts")
}

fn partial_path(dest: &Path) -> PathBuf {
    sibling(dest, ".partial")
}

fn sibling(dest: &Path, suffix: &str) -> PathBuf {
    let mut s = dest.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

fn load_manifest(dest: &Path) -> Option<Manifest> {
    let txt = fs::read_to_string(parts_dir(dest).join("manifest.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&txt).ok()?;
    let url = v.get("url")?.as_str()?.to_string();
    let total = v.get("total")?.as_u64()?;
    let part_size = v.get("part_size")?.as_u64()?;
    let n = part_count(total, part_size);
    let mut done = vec![None; n];
    if let Some(arr) = v.get("parts").and_then(|p| p.as_array()) {
        for p in arr {
            let i = p.get("i").and_then(|x| x.as_u64()).unwrap_or(u64::MAX) as usize;
            let ok = p.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
            let crc = p.get("crc").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
            if ok && i < done.len() && part_file(dest, i).is_file() {
                let expect = part_len(total, part_size, i);
                if part_file(dest, i).metadata().map(|m| m.len()).unwrap_or(0) == expect {
                    done[i] = Some(crc);
                }
            }
        }
    }
    Some(Manifest {
        url,
        total,
        part_size,
        done,
    })
}

fn save_manifest(dest: &Path, man: &Manifest) -> Result<(), String> {
    let dir = parts_dir(dest);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let parts: Vec<serde_json::Value> = man
        .done
        .iter()
        .enumerate()
        .filter_map(|(i, crc)| {
            crc.map(|c| serde_json::json!({ "i": i, "crc": c, "ok": true }))
        })
        .collect();
    let body = serde_json::json!({
        "url": man.url,
        "total": man.total,
        "part_size": man.part_size,
        "parts": parts,
    });
    fs::write(dir.join("manifest.json"), body.to_string()).map_err(|e| e.to_string())
}

fn part_count(total: u64, part_size: u64) -> usize {
    ((total + part_size - 1) / part_size) as usize
}

fn part_len(total: u64, part_size: u64, index: usize) -> u64 {
    let start = index as u64 * part_size;
    (total - start).min(part_size)
}

fn part_file(dest: &Path, index: usize) -> PathBuf {
    parts_dir(dest).join(format!("{index:05}.part"))
}

fn finish_parts(
    url: &str,
    dest: &Path,
    expected: Option<&str>,
    mut man: Manifest,
    on_bytes: &mut impl FnMut(u64),
) -> Result<u64, String> {
    let n = part_count(man.total, man.part_size);
    if man.done.len() != n {
        man.done.resize(n, None);
    }
    // Credit bytes we already have so a resumed install doesn't look stuck at 0.
    for (i, crc) in man.done.iter().enumerate() {
        if crc.is_some() {
            on_bytes(part_len(man.total, man.part_size, i));
        }
    }
    for i in 0..n {
        if man.done[i].is_some() {
            continue;
        }
        let bytes = fetch_part_retry(url, &man, i)?;
        let crc = crc32(&bytes);
        fs::write(part_file(dest, i), &bytes).map_err(|e| e.to_string())?;
        man.done[i] = Some(crc);
        save_manifest(dest, &man)?;
        on_bytes(bytes.len() as u64);
    }
    assemble(dest, &man)?;
    if hash_ok(dest, expected) {
        let _ = fs::remove_dir_all(parts_dir(dest));
        return Ok(man.total);
    }
    // Whole-file hash failed: a part is corrupt. Re-fetch each one and replace
    // only the parts whose bytes no longer match what we stored.
    let mut replaced = false;
    for i in 0..n {
        let fresh = fetch_part_retry(url, &man, i)?;
        let crc = crc32(&fresh);
        if man.done[i] != Some(crc) {
            fs::write(part_file(dest, i), &fresh).map_err(|e| e.to_string())?;
            man.done[i] = Some(crc);
            replaced = true;
            on_bytes(fresh.len() as u64);
        }
    }
    save_manifest(dest, &man)?;
    assemble(dest, &man)?;
    if hash_ok(dest, expected) {
        let _ = fs::remove_dir_all(parts_dir(dest));
        return Ok(man.total);
    }
    if !replaced {
        // Every stored part matched a fresh download, but the hash still fails.
        // Replace all of them once more (the source may have changed) and stop.
        for i in 0..n {
            let fresh = fetch_part_retry(url, &man, i)?;
            man.done[i] = Some(crc32(&fresh));
            fs::write(part_file(dest, i), &fresh).map_err(|e| e.to_string())?;
        }
        save_manifest(dest, &man)?;
        assemble(dest, &man)?;
        if hash_ok(dest, expected) {
            let _ = fs::remove_dir_all(parts_dir(dest));
            return Ok(man.total);
        }
    }
    let _ = fs::remove_file(dest);
    Err(format!(
        "{url}: checksum mismatch after replacing corrupt parts — completed parts were kept, retry to continue"
    ))
}

fn fetch_part_retry(url: &str, man: &Manifest, index: usize) -> Result<Vec<u8>, String> {
    let start = index as u64 * man.part_size;
    let len = part_len(man.total, man.part_size, index);
    let end = start + len - 1;
    let mut last = String::new();
    for attempt in 0..PART_TRIES {
        match fetch_range(url, start, end, len) {
            Ok(buf) => return Ok(buf),
            Err(e) => {
                last = format!("part {}/{}: {e}", index + 1, part_count(man.total, man.part_size));
                backoff(attempt);
            }
        }
    }
    Err(format!(
        "{url}: {last} — earlier parts were kept, retry to continue from part {}/{}",
        index + 1,
        part_count(man.total, man.part_size)
    ))
}

fn assemble(dest: &Path, man: &Manifest) -> Result<(), String> {
    let tmp = sibling(dest, ".assembling");
    let mut out = File::create(&tmp).map_err(|e| e.to_string())?;
    let n = part_count(man.total, man.part_size);
    for i in 0..n {
        let bytes = fs::read(part_file(dest, i)).map_err(|e| format!("part {i}: {e}"))?;
        let expect = part_len(man.total, man.part_size, i);
        if bytes.len() as u64 != expect {
            let _ = fs::remove_file(&tmp);
            return Err(format!("part {} is incomplete ({} != {expect})", i + 1, bytes.len()));
        }
        out.write_all(&bytes).map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())?;
    drop(out);
    fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
    Ok(())
}

fn download_stream(
    url: &str,
    dest: &Path,
    expected: Option<&str>,
    total: Option<u64>,
    on_bytes: &mut impl FnMut(u64),
) -> Result<u64, String> {
    let partial = partial_path(dest);
    let mut last = String::new();
    for attempt in 0..PART_TRIES {
        let have = fs::metadata(&partial).map(|m| m.len()).unwrap_or(0);
        if have > 0 && attempt == 0 {
            on_bytes(have);
        }
        let range = if have > 0 {
            Some(format!("bytes={have}-"))
        } else {
            None
        };
        match call(url, range.as_deref()) {
            Ok((_, resp)) => {
                let status = resp.status();
                let restart = status != 206;
                let mut file = if !restart && have > 0 {
                    OpenOptions::new()
                        .append(true)
                        .open(&partial)
                        .map_err(|e| e.to_string())?
                } else {
                    File::create(&partial).map_err(|e| e.to_string())?
                };
                let mut reader = resp.into_reader();
                let mut buf = vec![0u8; 64 * 1024];
                let mut failed = false;
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            if file.write_all(&buf[..n]).is_err() {
                                failed = true;
                                last = "failed to write partial".into();
                                break;
                            }
                            on_bytes(n as u64);
                        }
                        Err(e) => {
                            failed = true;
                            last = format!("{url}: {e}");
                            break;
                        }
                    }
                }
                let _ = file.flush();
                drop(file);
                let got = fs::metadata(&partial).map(|m| m.len()).unwrap_or(0);
                let complete = match total {
                    Some(t) => got == t,
                    None => !failed && got > 0,
                };
                if complete && (expected.is_none() || hash_ok(&partial, expected)) {
                    fs::rename(&partial, dest).map_err(|e| e.to_string())?;
                    return Ok(got);
                }
                if complete && expected.is_some() && !hash_ok(&partial, expected) {
                    // The whole small file arrived but is corrupt. Drop it and
                    // fetch it again — there is only one part.
                    let _ = fs::remove_file(&partial);
                    last = format!("{url}: checksum mismatch, retrying");
                    continue;
                }
                if failed {
                    backoff(attempt);
                    continue;
                }
                last = format!("{url}: incomplete download ({got} bytes)");
                backoff(attempt);
            }
            Err(e) => {
                last = e;
                backoff(attempt);
            }
        }
    }
    Err(format!("{last} — partial file was kept, retry to continue"))
}

fn probe(url: &str) -> Result<(u64, bool), String> {
    let (_, resp) = call(url, Some("bytes=0-0"))?;
    let status = resp.status();
    let ranged = status == 206;
    let total = if ranged {
        resp.header("Content-Range")
            .and_then(parse_total)
            .ok_or_else(|| "missing Content-Range".to_string())?
    } else {
        resp.header("Content-Length")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    };
    let _ = resp.into_reader().read_to_end(&mut Vec::new());
    if total == 0 {
        return Err("could not determine file size".into());
    }
    Ok((total, ranged))
}

fn fetch_range(url: &str, start: u64, end: u64, expect: u64) -> Result<Vec<u8>, String> {
    let spec = format!("bytes={start}-{end}");
    let (_, resp) = call(url, Some(&spec))?;
    let status = resp.status();
    let mut buf = Vec::new();
    resp.into_reader()
        .take(expect + 1)
        .read_to_end(&mut buf)
        .map_err(|e| e.to_string())?;
    if buf.len() as u64 == expect && (status == 206 || status == 200) {
        return Ok(buf);
    }
    Err(format!(
        "status {status}, got {} of {expect} bytes",
        buf.len()
    ))
}

fn parse_total(header: &str) -> Option<u64> {
    header.rsplit('/').next()?.trim().parse().ok()
}

fn call(url: &str, range: Option<&str>) -> Result<(String, ureq::Response), String> {
    let mut current = url.to_string();
    let mut last = String::new();
    for _ in 0..8 {
        let mut req = direct_agent().get(&current);
        if let Some(r) = range {
            req = req.set("Range", r);
        }
        match req.call() {
            Ok(resp) => return Ok((current, resp)),
            Err(ureq::Error::Status(code, resp)) if matches!(code, 301 | 302 | 303 | 307 | 308) => {
                let loc = resp.header("Location").unwrap_or("").to_string();
                if loc.is_empty() {
                    return Err(format!("redirect {code} without Location"));
                }
                current = join_url(&current, &loc);
            }
            Err(ureq::Error::Status(code, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                let snippet: String = body.chars().take(140).collect();
                return Err(format!("{current}: HTTP {code} {snippet}"));
            }
            Err(e) => {
                last = format!("{current}: {e}");
                // a dropped connection is retryable by the caller
                return Err(last);
            }
        }
    }
    Err(if last.is_empty() {
        "too many redirects".into()
    } else {
        last
    })
}

fn join_url(base: &str, loc: &str) -> String {
    if loc.starts_with("http://") || loc.starts_with("https://") {
        return loc.to_string();
    }
    if let Some(scheme) = base.find("://") {
        if loc.starts_with('/') {
            let rest = &base[scheme + 3..];
            let host_end = rest.find('/').map(|i| scheme + 3 + i).unwrap_or(base.len());
            return format!("{}{}", &base[..host_end], loc);
        }
    }
    format!("{base}/{loc}")
}

fn backoff(attempt: u32) {
    let ms = (400u64 * (1 << attempt.min(4))).min(6_000);
    std::thread::sleep(Duration::from_millis(ms));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    struct Hit {
        drops: AtomicUsize,
        bad: AtomicUsize,
    }

    fn serve(body: Vec<u8>, hits: Arc<Hit>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut buf = [0u8; 2048];
                let n = stream.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]);
                let range = req.lines().find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("range: bytes=")
                        .map(|s| s.trim().to_string())
                });
                let (start, end) = if let Some(r) = &range {
                    let mut it = r.split('-');
                    let a: u64 = it.next().unwrap_or("0").parse().unwrap_or(0);
                    let b: u64 = it.next().unwrap_or("0").parse().unwrap_or(0);
                    (a, b)
                } else {
                    (0, body.len().saturating_sub(1) as u64)
                };
                let end = end.min(body.len().saturating_sub(1) as u64);
                let slice = &body[start as usize..=end as usize];
                // First ranged hit of the middle part: drop the connection (internet cut).
                if start > 0 && start < body.len() as u64 / 2 && hits.drops.fetch_add(1, Ordering::SeqCst) == 0 {
                    let _ = stream.write_all(b"HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\n\r\nnope");
                    continue;
                }
                // Next hit of that same part: full length, wrong bytes (corrupt part).
                let payload = if start > 0 && start < body.len() as u64 / 2 && hits.bad.fetch_add(1, Ordering::SeqCst) == 0
                {
                    vec![0xA5u8; slice.len()]
                } else {
                    slice.to_vec()
                };
                let header = format!(
                    "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{end}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len(),
                    payload.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(&payload);
            }
        });
        format!("http://{addr}/file.bin")
    }

    #[test]
    fn resumes_after_dropped_and_corrupt_part() {
        let body: Vec<u8> = (0..300u16).map(|i| (i % 251) as u8).collect();
        let expect = sha1_hex(&body);
        let hits = Arc::new(Hit {
            drops: AtomicUsize::new(0),
            bad: AtomicUsize::new(0),
        });
        let url = serve(body.clone(), hits.clone());
        let dir = std::env::temp_dir().join(format!("rus-dl-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("file.bin");
        let n = download_inner(&url, &dest, Some(&expect), 64, |_| {}).expect("download");
        assert_eq!(n, body.len() as u64);
        assert_eq!(fs::read(&dest).unwrap(), body);
        assert!(hits.drops.load(Ordering::SeqCst) >= 1);
        assert!(hits.bad.load(Ordering::SeqCst) >= 1);
        let _ = fs::remove_dir_all(&dir);
    }
}
