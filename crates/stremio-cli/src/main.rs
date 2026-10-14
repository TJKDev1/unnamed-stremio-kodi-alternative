//! `stremio-cli`: query real Stremio addons from the terminal (T0.S3.6).
//!
//! ```text
//! stremio-cli manifest  ADDON...
//! stremio-cli catalog   ADDON... [-- TYPE ID [NAME=VALUE]...]   (no TYPE/ID: every catalog)
//! stremio-cli meta      ADDON... -- TYPE ID
//! stremio-cli streams   ADDON... -- TYPE ID
//! stremio-cli subtitles ADDON... -- TYPE ID
//! ```
//!
//! `ADDON` is a manifest URL. Addon URLs often hold secrets, so they can also come from
//! `--addons FILE` (one URL per line) or the `STREMIO_ADDONS` environment variable
//! (newline-separated), and the CLI never prints them: addons are shown by name.
//! Results print per addon **as each one answers**.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use app_addons_stremio::{
    Descriptor, Error, Fetched, Planned, ReqwestTransport, ResourcePath, ResourceResponse, Stream,
    StreamSource, Validated, fetch_all, fetch_manifest, plan, plan_catalogs,
};
use futures::StreamExt;
use futures::stream::FuturesOrdered;

const USAGE: &str = "usage:
  stremio-cli manifest  ADDON...
  stremio-cli catalog   ADDON... [-- TYPE ID [NAME=VALUE]...]
  stremio-cli meta      ADDON... -- TYPE ID
  stremio-cli streams   ADDON... -- TYPE ID
  stremio-cli subtitles ADDON... -- TYPE ID

options:
  --addons FILE    read addon URLs from FILE, one per line
  --timeout SECS   per-request timeout (default 15)
  --limit N        items to print per addon (default 5, 0 = all)

Addon URLs are also read from STREMIO_ADDONS (newline-separated). URLs are never printed.";

struct Args {
    command: String,
    addons: Vec<String>,
    target: Vec<String>,
    timeout: Duration,
    limit: usize,
}

fn parse_args() -> Result<Args, String> {
    let mut raw = std::env::args().skip(1);
    let command = raw.next().ok_or("missing command")?;
    let mut args = Args {
        command,
        addons: Vec::new(),
        target: Vec::new(),
        timeout: Duration::from_secs(15),
        limit: 5,
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
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}\n\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    match run(args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Args) -> Result<(), String> {
    let transport = ReqwestTransport::new(args.timeout).map_err(|e| e.to_string())?;
    let start = Instant::now();
    let addons = load_addons(&transport, &args.addons, start).await;
    if args.command == "manifest" {
        return Ok(());
    }
    if addons.is_empty() {
        return Err("no addon loaded".into());
    }

    let resource = match args.command.as_str() {
        "catalog" => "catalog",
        "meta" => "meta",
        "streams" => "stream",
        "subtitles" => "subtitles",
        other => return Err(format!("unknown command `{other}`\n\n{USAGE}")),
    };
    let planned = match (resource, args.target.as_slice()) {
        ("catalog", []) => plan_catalogs(&addons, None, &[]),
        (_, [r#type, id, extra @ ..]) => {
            let mut path = ResourcePath::new(resource, r#type, id);
            for pair in extra {
                let (name, value) = pair
                    .split_once('=')
                    .ok_or(format!("extra `{pair}` is not NAME=VALUE"))?;
                path = path.with_extra(name, value);
            }
            plan(&addons, &path)
        }
        _ => return Err(format!("`{}` needs `-- TYPE ID`", args.command)),
    };
    if planned.is_empty() {
        return Err("no loaded addon serves that request".into());
    }
    println!("asking {} addon request(s)…", planned.len());

    let started = Instant::now();
    let mut results = fetch_all(&transport, &addons, planned);
    while let Some((planned, result)) = results.next().await {
        print_result(&addons, &planned, result, started, args.limit);
    }
    Ok(())
}

/// Fetches every manifest at once; prints each as it loads. Keeps the given order.
async fn load_addons(
    transport: &ReqwestTransport,
    urls: &[String],
    start: Instant,
) -> Vec<Descriptor> {
    let mut loading: FuturesOrdered<_> = urls
        .iter()
        .map(|url| async move { (url, fetch_manifest(transport, url).await) })
        .collect();
    let mut addons = Vec::new();
    let mut n = 0;
    while let Some((url, result)) = loading.next().await {
        n += 1;
        match result {
            Ok(Validated { manifest, warnings }) => {
                println!(
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
                );
                for w in warnings {
                    println!("    warning: {}", w.0);
                }
                addons.push(Descriptor {
                    transport_url: url.clone(),
                    manifest,
                    other: Default::default(),
                });
            }
            Err(e) => println!("{} addon #{n}: {e}", elapsed(start)),
        }
    }
    addons
}

fn print_result(
    addons: &[Descriptor],
    planned: &Planned,
    result: Result<Fetched, Error>,
    started: Instant,
    limit: usize,
) {
    let name = &addons[planned.addon].manifest.name;
    let what = format!("{}/{}", planned.path.r#type, planned.path.id);
    let fetched = match result {
        Ok(f) => f,
        Err(e) => {
            println!("{} {name} {what}: {e}", elapsed(started));
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
                        "video {} S{:?}E{:?} {}",
                        v.id,
                        v.season.unwrap_or_default(),
                        v.episode.unwrap_or_default(),
                        v.title.as_deref().unwrap_or("")
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
    println!(
        "{} {name} {label}: {} item(s){}",
        elapsed(started),
        lines.len(),
        if fetched.dropped.is_empty() {
            String::new()
        } else {
            format!(", {} dropped", fetched.dropped.len())
        }
    );
    for line in lines.iter().take(limit) {
        println!("    {line}");
    }
    for d in &fetched.dropped {
        println!("    dropped {}: {}", d.item, d.error);
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
