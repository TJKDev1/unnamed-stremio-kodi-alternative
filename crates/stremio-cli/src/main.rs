//! `stremio-cli`: query real Stremio addons from the terminal (T0.S3.6).
//!
//! ```text
//! stremio-cli manifest  ADDON...
//! stremio-cli catalog   ADDON... [-- TYPE ID [NAME=VALUE]...]   (no TYPE/ID: every catalog)
//! stremio-cli meta      ADDON... -- TYPE ID
//! stremio-cli streams   ADDON... -- TYPE ID
//! stremio-cli subtitles ADDON... -- TYPE ID
//! stremio-cli record --name NAME ADDON [-- RESOURCE/TYPE/ID[/NAME=VALUE&...]...]
//! ```
//!
//! `record` saves scrubbed, anonymized responses to `fixtures/stremio/NAME/` (see [`record`]);
//! without explicit paths it picks catalogs, metas and streams from the manifest.
//!
//! `ADDON` is a manifest URL. Addon URLs often hold secrets, so they can also come from
//! `--addons FILE` (one URL per line) or the `STREMIO_ADDONS` environment variable
//! (newline-separated), and the CLI never prints them: addons are shown by name, and every
//! printed line goes through the fixture scrubber. Each addon's requests start as soon as its
//! own manifest loads, and results print **as each one answers**.

mod record;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use app_addons_stremio::{
    Descriptor, Error, Fetched, Planned, ReqwestTransport, ResourcePath, ResourceResponse, Stream,
    StreamSource, Validated, fetch, fetch_manifest, manifest, plan, plan_catalogs,
};
use app_fixtures::Scrubber;
use futures::future::{FutureExt, LocalBoxFuture};
use futures::stream::{FuturesUnordered, StreamExt};

const USAGE: &str = "usage:
  stremio-cli manifest  ADDON...
  stremio-cli catalog   ADDON... [-- TYPE ID [NAME=VALUE]...]
  stremio-cli meta      ADDON... -- TYPE ID
  stremio-cli streams   ADDON... -- TYPE ID
  stremio-cli subtitles ADDON... -- TYPE ID
  stremio-cli record --name NAME ADDON [-- RESOURCE/TYPE/ID[/NAME=VALUE&...]...]

options:
  --addons FILE    read addon URLs from FILE, one per line
  --timeout SECS   per-request timeout (default 15)
  --limit N        items to print per addon (default 5, 0 = all)
  --name NAME      record: fixture folder name (fixtures/stremio/NAME)
  --out DIR        record: parent folder (default fixtures/stremio)

record also reads FIXTURE_SECRETS (comma- or newline-separated) like app-fixtures scrub.

Addon URLs are also read from STREMIO_ADDONS (newline-separated). URLs are never printed.";

struct Args {
    command: String,
    addons: Vec<String>,
    target: Vec<String>,
    timeout: Duration,
    limit: usize,
    name: Option<String>,
    out: PathBuf,
}

fn parse_args(raw: Vec<String>) -> Result<Args, String> {
    let mut raw = raw.into_iter();
    let command = raw.next().ok_or("missing command")?;
    let mut args = Args {
        command,
        addons: Vec::new(),
        target: Vec::new(),
        timeout: Duration::from_secs(15),
        limit: 5,
        name: None,
        out: PathBuf::from("fixtures/stremio"),
    };
    if let Ok(env) = std::env::var("STREMIO_ADDONS") {
        args.addons.extend(urls_from_lines(&env));
    }
    let mut after_dashes = false;
    while let Some(arg) = raw.next() {
        if after_dashes {
            args.target.push(arg);
            continue;
        }
        let mut value = |name: &str| raw.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--" => after_dashes = true,
            "--addons" => {
                let file = value("--addons")?;
                let text = std::fs::read_to_string(&file).map_err(|e| format!("{file}: {e}"))?;
                args.addons.extend(urls_from_lines(&text));
            }
            "--timeout" => {
                let secs: f64 = value("--timeout")?.parse().map_err(|_| "bad --timeout")?;
                args.timeout = Duration::from_secs_f64(secs);
            }
            "--limit" => {
                args.limit = value("--limit")?.parse().map_err(|_| "bad --limit")?;
                if args.limit == 0 {
                    args.limit = usize::MAX;
                }
            }
            "--name" => args.name = Some(value("--name")?),
            "--out" => args.out = PathBuf::from(value("--out")?),
            _ if arg.starts_with("--") => return Err(format!("unknown option {arg}")),
            _ => args.addons.push(arg),
        }
    }
    if args.addons.is_empty() {
        return Err("no addons given".into());
    }
    Ok(args)
}

fn urls_from_lines(text: &str) -> impl Iterator<Item = String> + '_ {
    urls_from_lines_str(text).map(str::to_owned)
}

fn urls_from_lines_str(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    // Built before anything can fail, from every URL-looking argument, so even argument
    // errors are printed scrubbed.
    let out = Printer::new(raw.iter().filter(|a| a.contains("://")));
    let args = match parse_args(raw) {
        Ok(args) => args,
        Err(e) => {
            out.err(&format!("{e}\n\n{USAGE}"));
            return ExitCode::FAILURE;
        }
    };
    // URLs from `--addons FILE` are only known now.
    let out = Printer::new(args.addons.iter());
    match run(args, &out).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            out.err(&e);
            ExitCode::FAILURE
        }
    }
}

/// What the query loop is waiting on: a manifest, or a resource answer.
enum Event {
    Manifest(usize, Result<(String, Validated), Error>),
    Response(Planned, Result<Fetched, Error>),
}

async fn run(args: Args, out: &Printer) -> Result<(), String> {
    let transport = ReqwestTransport::new(args.timeout).map_err(|e| e.to_string())?;
    if args.command == "record" {
        return record(&transport, args).await;
    }

    let resource = match args.command.as_str() {
        "manifest" => None,
        "catalog" => Some("catalog"),
        "meta" => Some("meta"),
        "streams" => Some("stream"),
        "subtitles" => Some("subtitles"),
        other => return Err(format!("unknown command `{other}`\n\n{USAGE}")),
    };
    // `None` with a resource: every catalog (the home board).
    let request = match (resource, args.target.as_slice()) {
        (None, _) | (Some("catalog"), []) => None,
        (Some(resource), [r#type, id, extra @ ..]) => {
            let mut path = ResourcePath::new(resource, r#type, id);
            for pair in extra {
                let (name, value) = pair
                    .split_once('=')
                    .ok_or(format!("extra `{pair}` is not NAME=VALUE"))?;
                path = path.with_extra(name, value);
            }
            Some(path)
        }
        _ => return Err(format!("`{}` needs `-- TYPE ID`", args.command)),
    };

    // One loop for everything: each addon's requests start as soon as *its* manifest
    // arrives, so a slow manifest never holds back another addon's results.
    let start = Instant::now();
    let transport = &transport;
    let mut pending: FuturesUnordered<LocalBoxFuture<'_, Event>> = FuturesUnordered::new();
    for (i, url) in args.addons.iter().enumerate() {
        pending.push(
            async move {
                let result = match manifest::transport_url(url) {
                    Ok(url) => fetch_manifest(transport, &url).await.map(|v| (url, v)),
                    Err(e) => Err(e),
                };
                Event::Manifest(i, result)
            }
            .boxed_local(),
        );
    }
    let mut names: Vec<Option<String>> = vec![None; args.addons.len()];
    let mut asked = 0;
    while let Some(event) = pending.next().await {
        match event {
            Event::Manifest(i, Err(e)) => {
                out.line(&format!("{} addon #{}: {e}", elapsed(start), i + 1));
            }
            Event::Manifest(i, Ok((url, Validated { manifest, warnings }))) => {
                out.line(&format!(
                    "{} {} {} ({}): {} catalog(s), resources [{}]",
                    elapsed(start),
                    manifest.name,
                    manifest.version,
                    manifest.id,
                    manifest.catalogs.len(),
                    manifest
                        .resources
                        .iter()
                        .map(|r| r.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                for w in warnings {
                    out.line(&format!("    warning: {}", w.0));
                }
                names[i] = Some(manifest.name.clone());
                let Some(resource) = resource else {
                    continue;
                };
                let addon = [Descriptor {
                    transport_url: url.clone(),
                    manifest,
                    other: Default::default(),
                }];
                let planned = match &request {
                    Some(path) => plan(&addon, path),
                    None if resource == "catalog" => plan_catalogs(&addon, None, &[]),
                    None => Vec::new(),
                };
                for planned in planned {
                    asked += 1;
                    let url = url.clone();
                    pending.push(
                        async move {
                            let result = fetch(transport, &url, &planned.path).await;
                            Event::Response(
                                Planned {
                                    addon: i,
                                    ..planned
                                },
                                result,
                            )
                        }
                        .boxed_local(),
                    );
                }
            }
            Event::Response(planned, result) => {
                let name = names[planned.addon].as_deref().unwrap_or("?");
                print_result(out, name, &planned, result, start, args.limit);
            }
        }
    }
    if resource.is_some() && asked == 0 {
        return Err("no loaded addon serves that request".into());
    }
    Ok(())
}

async fn record(transport: &ReqwestTransport, args: Args) -> Result<(), String> {
    let name = args.name.ok_or("record needs --name")?;
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("--name must be letters, digits and dashes".into());
    }
    let [url] = args.addons.as_slice() else {
        return Err("record takes exactly one addon".into());
    };
    let paths = args
        .target
        .iter()
        .map(|p| record::parse_path(p))
        .collect::<Result<Vec<_>, _>>()?;
    record::Recorder::new(transport, url, &args.out, &name)?
        .run(paths)
        .await
}

/// Prints lines with anything secret-looking removed: errors can quote URLs, and addons
/// control every string they send.
struct Printer(Scrubber);

impl Printer {
    /// Knows `addons` (and their query values), `STREMIO_ADDONS` and `FIXTURE_SECRETS`.
    fn new<'a>(addons: impl Iterator<Item = &'a String>) -> Self {
        let mut scrubber = Scrubber::new();
        let env_addons = std::env::var("STREMIO_ADDONS").unwrap_or_default();
        for url in addons {
            scrubber = scrubber.addon_url(url);
        }
        for url in urls_from_lines_str(&env_addons) {
            scrubber = scrubber.addon_url(url);
        }
        if let Ok(env) = std::env::var("FIXTURE_SECRETS") {
            for secret in env.split([',', '\n']).map(str::trim) {
                scrubber = scrubber.secret(secret);
            }
        }
        Self(scrubber)
    }

    fn line(&self, text: &str) {
        println!("{}", self.0.scrub_str(text).0);
    }

    fn err(&self, text: &str) {
        eprintln!("{}", self.0.scrub_str(text).0);
    }
}

fn print_result(
    out: &Printer,
    name: &str,
    planned: &Planned,
    result: Result<Fetched, Error>,
    started: Instant,
    limit: usize,
) {
    let what = format!("{}/{}", planned.path.r#type, planned.path.id);
    let fetched = match result {
        Ok(f) => f,
        Err(e) => {
            out.line(&format!("{} {name} {what}: {e}", elapsed(started)));
            return;
        }
    };
    let lines: Vec<String> = match &fetched.response {
        ResourceResponse::Catalog(r) => r
            .metas
            .iter()
            .map(|m| format!("{} {}", m.id, m.name.as_deref().unwrap_or("?")))
            .collect(),
        ResourceResponse::Meta(r) => match &r.meta {
            Some(m) => {
                let p = &m.preview;
                let mut lines = vec![format!(
                    "{} {} ({})",
                    p.id,
                    p.name.as_deref().unwrap_or("?"),
                    p.release_info.as_deref().unwrap_or("-")
                )];
                lines.extend(m.videos.iter().map(|v| {
                    format!(
                        "video {} S{}E{} {}",
                        v.id,
                        v.season.unwrap_or_default(),
                        v.episode_number().unwrap_or_default(),
                        v.display_title().unwrap_or("")
                    )
                }));
                lines
            }
            None => vec!["(no meta)".into()],
        },
        ResourceResponse::Streams(r) => r.streams.iter().map(describe_stream).collect(),
        ResourceResponse::Subtitles(r) => r
            .subtitles
            .iter()
            .map(|s| format!("{} {}", s.lang, s.id.as_deref().unwrap_or("")))
            .collect(),
        ResourceResponse::AddonCatalog(r) => {
            r.addons.iter().map(|a| a.manifest.name.clone()).collect()
        }
    };
    let label = match planned.path.resource.as_str() {
        "catalog" => format!("{} ({})", planned.path.id, planned.path.r#type),
        _ => what,
    };
    out.line(&format!(
        "{} {name} {label}: {} item(s){}",
        elapsed(started),
        lines.len(),
        if fetched.dropped.is_empty() {
            String::new()
        } else {
            format!(", {} dropped", fetched.dropped.len())
        }
    ));
    for line in lines.iter().take(limit) {
        out.line(&format!("    {line}"));
    }
    for d in &fetched.dropped {
        out.line(&format!("    dropped {}: {}", d.item, d.error));
    }
}

fn describe_stream(s: &Stream) -> String {
    let kind = match s.source() {
        StreamSource::Url(_) => "url",
        StreamSource::YouTube(_) => "youtube",
        StreamSource::Torrent { .. } => "torrent",
        StreamSource::External(_) => "external",
        StreamSource::Nzb(_) => "nzb (requires plugin)",
        StreamSource::Archive(_) => "archive (requires plugin)",
        StreamSource::Unknown => "unknown (requires plugin)",
    };
    format!(
        "[{kind}] {} | {}",
        first_line(s.name.as_deref()),
        first_line(s.title.as_deref().or(s.description.as_deref()))
    )
}

fn elapsed(since: Instant) -> String {
    format!("[{:>5} ms]", since.elapsed().as_millis())
}

fn first_line(s: Option<&str>) -> &str {
    s.and_then(|s| s.lines().next()).unwrap_or("")
}
