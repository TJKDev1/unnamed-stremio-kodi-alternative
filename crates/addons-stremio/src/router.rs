//! Which addons to ask, and asking them all at once (spec: Router).

use futures::stream::{FuturesUnordered, Stream};

use crate::model::{Descriptor, Manifest, Meta, Parsed, ResourceResponse, parse_resource};
use crate::route::ResourcePath;
use crate::transport::Transport;
use crate::{Dropped, Error};

/// One planned request: which addon (index into the list given to the planner) and what path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    pub addon: usize,
    pub path: ResourcePath,
}

impl Manifest {
    /// Whether this addon can answer `path`. Catalogs are matched against `catalogs` (type, id
    /// and extras), addon catalogs against `addonCatalogs`; everything else by resource name,
    /// type and id prefix, where a resource object's own `types`/`idPrefixes` win over the
    /// manifest's and no `idPrefixes` at all means any id.
    pub fn supports(&self, path: &ResourcePath) -> bool {
        match path.resource.as_str() {
            "catalog" => self.catalogs.iter().any(|c| {
                c.r#type == path.r#type
                    && c.id == path.id
                    && c.is_extra_supported(&path.extra_names())
            }),
            "addon_catalog" => self
                .addon_catalogs
                .iter()
                .any(|c| c.r#type == path.r#type && c.id == path.id),
            resource => self.resources.iter().any(|r| {
                if r.name() != resource {
                    return false;
                }
                let types = r.types().unwrap_or(&self.types);
                let prefixes = r.id_prefixes().or(self.id_prefixes.as_deref());
                types.contains(&path.r#type)
                    && prefixes.is_none_or(|p| p.iter().any(|p| path.id.starts_with(p.as_str())))
            }),
        }
    }
}

/// The addons (in install order) that can answer `path`.
pub fn plan(addons: &[Descriptor], path: &ResourcePath) -> Vec<Planned> {
    addons
        .iter()
        .enumerate()
        .filter(|(_, a)| a.manifest.supports(path))
        .map(|(addon, _)| Planned {
            addon,
            path: path.clone(),
        })
        .collect()
}

/// Every catalog (optionally of one type) that accepts exactly these extras, across all addons,
/// in install order and then manifest order. Board rows on a home screen come from this.
pub fn plan_catalogs(
    addons: &[Descriptor],
    r#type: Option<&str>,
    extra: &[(String, String)],
) -> Vec<Planned> {
    let probe = ResourcePath {
        extra: extra.to_vec(),
        ..ResourcePath::new("catalog", "", "")
    };
    let names = probe.extra_names();
    let mut out = Vec::new();
    for (addon, a) in addons.iter().enumerate() {
        for c in &a.manifest.catalogs {
            if r#type.is_none_or(|t| t == c.r#type) && c.is_extra_supported(&names) {
                out.push(Planned {
                    addon,
                    path: ResourcePath {
                        extra: extra.to_vec(),
                        ..ResourcePath::new("catalog", &c.r#type, &c.id)
                    },
                });
            }
        }
    }
    out
}

/// Streams embedded in a meta's video (`videos[].streams`). When present, other stream addons
/// are not asked for that video (spec: Behaviour).
pub fn embedded_streams<'a>(meta: &'a Meta, video_id: &str) -> Option<&'a [crate::Stream]> {
    meta.videos
        .iter()
        .find(|v| v.id == video_id && !v.streams.is_empty())
        .map(|v| v.streams.as_slice())
}

/// A successful answer from one addon.
#[derive(Debug, Clone, PartialEq)]
pub struct Fetched {
    pub response: ResourceResponse,
    /// List items that failed to parse and were dropped.
    pub dropped: Vec<Dropped>,
    /// The `Cache-Control` header, used when the body has no caching hints.
    pub cache_control: Option<String>,
}

/// Fetches one resource from the addon at `transport_url`.
pub async fn fetch(
    transport: &impl Transport,
    transport_url: &str,
    path: &ResourcePath,
) -> Result<Fetched, Error> {
    let response = transport
        .get(&path.url(transport_url))
        .await
        .map_err(Error::Transport)?;
    if !(200..300).contains(&response.status) {
        return Err(Error::Status(response.status));
    }
    let Parsed { value, dropped } =
        parse_resource(&path.resource, &response.body).map_err(Error::Parse)?;
    Ok(Fetched {
        response: value,
        dropped,
        cache_control: response.cache_control,
    })
}

/// Runs every planned request at once and yields `(planned, result)` **in completion order**:
/// a slow addon never holds back a fast one. Timeouts are the transport's (D60), so this never
/// names a runtime.
pub fn fetch_all<'a, T: Transport>(
    transport: &'a T,
    addons: &'a [Descriptor],
    plan: Vec<Planned>,
) -> impl Stream<Item = (Planned, Result<Fetched, Error>)> + 'a {
    plan.into_iter()
        .map(|planned| async move {
            let url = &addons[planned.addon].transport_url;
            let result = fetch(transport, url, &planned.path).await;
            (planned, result)
        })
        .collect::<FuturesUnordered<_>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{HttpResponse, TransportError};
    use futures::StreamExt;
    use futures::channel::oneshot;
    use serde_json::json;
    use std::sync::Mutex;

    fn addon(url: &str, manifest: serde_json::Value) -> Descriptor {
        Descriptor {
            transport_url: url.into(),
            manifest: serde_json::from_value(manifest).unwrap(),
            other: Default::default(),
        }
    }

    fn addons() -> Vec<Descriptor> {
        vec![
            addon(
                "https://cinemeta/manifest.json",
                json!({
                    "id": "cinemeta", "name": "C", "version": "1", "types": ["movie", "series"],
                    "idPrefixes": ["tt"], "resources": ["catalog", "meta", "addon_catalog"],
                    "catalogs": [
                        { "type": "movie", "id": "top", "extra": [{ "name": "search" }, { "name": "skip" }] },
                        { "type": "series", "id": "top", "extra": [{ "name": "search", "isRequired": true }] }
                    ],
                    "addonCatalogs": [{ "type": "all", "id": "official" }]
                }),
            ),
            addon(
                "https://kitsu/manifest.json",
                json!({
                    "id": "kitsu", "name": "K", "version": "1", "types": ["anime", "movie", "series"],
                    "resources": ["catalog", { "name": "meta", "types": ["anime", "movie", "series"], "idPrefixes": ["kitsu:"] }],
                    "catalogs": []
                }),
            ),
            addon(
                "https://torrentio/manifest.json",
                json!({
                    "id": "torrentio", "name": "T", "version": "1", "types": ["movie", "series"],
                    "idPrefixes": ["tt", "kitsu"], "resources": ["stream"], "catalogs": []
                }),
            ),
            addon(
                "https://subs/manifest.json",
                json!({
                    "id": "subs", "name": "S", "version": "1", "types": ["movie", "series"],
                    "resources": ["subtitles"], "catalogs": []
                }),
            ),
        ]
    }

    fn picked(plan: Vec<Planned>) -> Vec<usize> {
        plan.into_iter().map(|p| p.addon).collect()
    }

    #[test]
    fn routes_by_resource_type_and_prefix() {
        let a = addons();
        assert_eq!(
            picked(plan(&a, &ResourcePath::new("meta", "movie", "tt1"))),
            [0]
        );
        assert_eq!(
            picked(plan(&a, &ResourcePath::new("meta", "series", "kitsu:1"))),
            [1]
        );
        assert_eq!(
            picked(plan(
                &a,
                &ResourcePath::new("stream", "series", "kitsu:1:2")
            )),
            [2]
        );
        assert!(plan(&a, &ResourcePath::new("stream", "tv", "tt1")).is_empty());
        // No idPrefixes: any id.
        assert_eq!(
            picked(plan(&a, &ResourcePath::new("subtitles", "movie", "x"))),
            [3]
        );
        assert_eq!(
            picked(plan(
                &a,
                &ResourcePath::new("addon_catalog", "all", "official")
            )),
            [0]
        );
    }

    #[test]
    fn catalogs_respect_extras() {
        let a = addons();
        let top = |extra: &[(&str, &str)]| {
            let mut p = ResourcePath::new("catalog", "movie", "top");
            for (n, v) in extra {
                p = p.with_extra(n, v);
            }
            p
        };
        assert_eq!(picked(plan(&a, &top(&[]))), [0]);
        assert_eq!(picked(plan(&a, &top(&[("skip", "100")]))), [0]);
        assert!(plan(&a, &top(&[("genre", "x")])).is_empty());
        // Series top needs search.
        let board = plan_catalogs(&a, None, &[]);
        assert_eq!(board.len(), 1);
        let search = plan_catalogs(&a, None, &[("search".into(), "dune".into())]);
        assert_eq!(search.len(), 2);
        assert_eq!(
            search[1].path.to_url_path(),
            "/catalog/series/top/search=dune.json"
        );
    }

    enum Answer {
        Now(u16, &'static str),
        Fail(&'static str),
        /// Answers when the test sends the body.
        Later(Mutex<Option<oneshot::Receiver<&'static str>>>),
    }

    struct Fake(Vec<(&'static str, Answer)>);

    impl Transport for Fake {
        async fn get(&self, url: &str) -> Result<HttpResponse, TransportError> {
            let (_, answer) = self
                .0
                .iter()
                .find(|(prefix, _)| url.starts_with(prefix))
                .unwrap_or_else(|| panic!("unexpected {url}"));
            let (status, body) = match answer {
                Answer::Now(status, body) => (*status, *body),
                Answer::Fail(e) => return Err(TransportError((*e).into())),
                Answer::Later(rx) => {
                    let rx = rx.lock().unwrap().take().unwrap();
                    (200, rx.await.unwrap())
                }
            };
            Ok(HttpResponse {
                status,
                cache_control: None,
                body: body.as_bytes().to_vec(),
            })
        }
    }

    #[test]
    fn results_arrive_in_completion_order_and_failures_are_per_addon() {
        let a = addons();
        let (tx, rx) = oneshot::channel();
        let transport = Fake(vec![
            ("https://torrentio/", Answer::Later(Mutex::new(Some(rx)))),
            (
                "https://cinemeta/",
                Answer::Now(200, r#"{"streams":[{"infoHash":"h"}]}"#),
            ),
            ("https://kitsu/", Answer::Now(503, "")),
            ("https://subs/", Answer::Fail("timed out")),
        ]);
        let path = ResourcePath::new("stream", "movie", "tt1");
        let plan = (0..4)
            .map(|addon| Planned {
                addon,
                path: path.clone(),
            })
            .collect();
        let mut stream = std::pin::pin!(fetch_all(&transport, &a, plan));
        futures::executor::block_on(async {
            let mut first = Vec::new();
            for _ in 0..3 {
                first.push(stream.next().await.unwrap());
            }
            // Torrentio is still waiting; everyone else has answered.
            assert!(first.iter().all(|(p, _)| p.addon != 2));
            let by = |i: usize| &first.iter().find(|(p, _)| p.addon == i).unwrap().1;
            assert!(matches!(by(0), Ok(f) if matches!(&f.response,
                ResourceResponse::Streams(s) if s.streams.len() == 1)));
            assert_eq!(by(1), &Err(Error::Status(503)));
            assert!(matches!(by(3), Err(Error::Transport(_))));

            tx.send(r#"{"streams":[]}"#).unwrap();
            let (last, result) = stream.next().await.unwrap();
            assert_eq!(last.addon, 2);
            assert!(result.is_ok());
            assert!(stream.next().await.is_none());
        });
    }

    #[test]
    fn embedded_streams_suppress_addons() {
        let meta: Meta = serde_json::from_value(json!({
            "id": "yt1", "type": "channel",
            "videos": [{ "id": "v1", "streams": [{ "ytId": "abc" }] }, { "id": "v2" }]
        }))
        .unwrap();
        assert_eq!(embedded_streams(&meta, "v1").unwrap().len(), 1);
        assert!(embedded_streams(&meta, "v2").is_none());
    }
}
