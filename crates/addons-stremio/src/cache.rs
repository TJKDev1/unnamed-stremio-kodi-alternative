//! `cacheMaxAge` / `staleRevalidate` / `staleError` (spec: Caching). In memory for the spike.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::Error;
use crate::model::CacheHints;
use crate::route::ResourcePath;
use crate::router::{Fetched, fetch};
use crate::transport::Transport;

/// Time source, injected so tests can use a fake.
pub trait Clock: Send + Sync {
    /// Time since any fixed point; only differences matter.
    fn now(&self) -> Duration;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
    }
}

/// How long a response may be used, in seconds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CachePolicy {
    pub max_age: u64,
    pub stale_revalidate: u64,
    pub stale_error: u64,
}

impl CachePolicy {
    /// Body hints win; without any, `Cache-Control` is used. `None` means "don't cache".
    pub fn from_response(hints: CacheHints, cache_control: Option<&str>) -> Option<Self> {
        let from_body = hints.cache_max_age.is_some()
            || hints.stale_revalidate.is_some()
            || hints.stale_error.is_some();
        let policy = if from_body {
            Self {
                max_age: hints.cache_max_age.unwrap_or(0),
                stale_revalidate: hints.stale_revalidate.unwrap_or(0),
                stale_error: hints.stale_error.unwrap_or(0),
            }
        } else {
            Self::from_cache_control(cache_control?)?
        };
        (policy != Self::default()).then_some(policy)
    }

    fn from_cache_control(header: &str) -> Option<Self> {
        let mut policy = Self::default();
        for directive in header.split(',') {
            let directive = directive.trim().to_ascii_lowercase();
            let (name, value) = directive.split_once('=').unwrap_or((&directive, ""));
            let secs = || value.trim_matches('"').parse::<u64>().ok();
            match name.trim() {
                "no-store" | "no-cache" => return None,
                "max-age" => policy.max_age = secs()?,
                "stale-while-revalidate" => policy.stale_revalidate = secs()?,
                "stale-if-error" => policy.stale_error = secs()?,
                _ => {}
            }
        }
        Some(policy)
    }

    /// What an entry of this `age` is good for.
    pub fn freshness(&self, age: Duration) -> Freshness {
        let age = age.as_secs_f64();
        let max_age = self.max_age as f64;
        if age < max_age {
            Freshness::Fresh
        } else if age < max_age + self.stale_revalidate as f64 {
            Freshness::Stale
        } else {
            Freshness::Expired {
                usable_on_error: age < max_age + self.stale_error as f64,
            }
        }
    }

    fn lifetime(&self) -> u64 {
        self.max_age + self.stale_revalidate.max(self.stale_error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// Serve it; don't ask the addon.
    Fresh,
    /// Serve it now and revalidate in the background.
    Stale,
    /// Fetch again; serve it only if that fails and `usable_on_error`.
    Expired { usable_on_error: bool },
}

/// Where a [`Cache::fetch`] answer came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Served {
    /// Fresh from the addon.
    Network,
    /// From cache, still fresh.
    Fresh,
    /// From cache, stale: the caller should call [`Cache::revalidate`] in the background.
    Stale,
    /// From cache after the addon failed, within `staleError`.
    StaleOnError,
}

#[derive(Debug)]
struct Entry {
    stored_at: Duration,
    policy: CachePolicy,
    fetched: Fetched,
}

/// Response cache keyed by full resource URL.
#[derive(Debug)]
pub struct Cache<C: Clock = SystemClock> {
    clock: C,
    entries: Mutex<HashMap<String, Entry>>,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new(SystemClock)
    }
}

impl<C: Clock> Cache<C> {
    pub fn new(clock: C) -> Self {
        Self {
            clock,
            entries: Mutex::default(),
        }
    }

    fn lookup(&self, url: &str) -> Option<(Fetched, Freshness)> {
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let entry = entries.get(url)?;
        let age = self.clock.now().saturating_sub(entry.stored_at);
        Some((entry.fetched.clone(), entry.policy.freshness(age)))
    }

    fn store(&self, url: String, fetched: &Fetched) {
        let Some(policy) =
            CachePolicy::from_response(fetched.response.cache(), fetched.cache_control.as_deref())
        else {
            return;
        };
        let now = self.clock.now();
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        // Drop entries that can no longer be served at all.
        entries.retain(|_, e| {
            now.saturating_sub(e.stored_at) < Duration::from_secs(e.policy.lifetime())
        });
        entries.insert(
            url,
            Entry {
                stored_at: now,
                policy,
                fetched: fetched.clone(),
            },
        );
    }

    /// Fetches `path` from the addon at `transport_url`, following the caching table.
    pub async fn fetch(
        &self,
        transport: &impl Transport,
        transport_url: &str,
        path: &ResourcePath,
    ) -> Result<(Fetched, Served), Error> {
        let url = path.url(transport_url);
        let cached = self.lookup(&url);
        let usable_on_error = match cached {
            Some((fetched, Freshness::Fresh)) => return Ok((fetched, Served::Fresh)),
            Some((fetched, Freshness::Stale)) => return Ok((fetched, Served::Stale)),
            Some((
                fetched,
                Freshness::Expired {
                    usable_on_error: true,
                },
            )) => Some(fetched),
            _ => None,
        };
        match fetch(transport, transport_url, path).await {
            Ok(fetched) => {
                self.store(url, &fetched);
                Ok((fetched, Served::Network))
            }
            Err(e) => match usable_on_error {
                Some(fetched) => Ok((fetched, Served::StaleOnError)),
                None => Err(e),
            },
        }
    }

    /// Re-fetches and stores `path`, ignoring what's cached. Run this after [`Served::Stale`].
    pub async fn revalidate(
        &self,
        transport: &impl Transport,
        transport_url: &str,
        path: &ResourcePath,
    ) -> Result<Fetched, Error> {
        let fetched = fetch(transport, transport_url, path).await?;
        self.store(path.url(transport_url), &fetched);
        Ok(fetched)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{HttpResponse, TransportError};
    use futures::executor::block_on;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

    #[derive(Default)]
    struct FakeClock(AtomicU64);
    impl Clock for &FakeClock {
        fn now(&self) -> Duration {
            Duration::from_secs(self.0.load(Ordering::SeqCst))
        }
    }
    impl FakeClock {
        fn set(&self, secs: u64) {
            self.0.store(secs, Ordering::SeqCst);
        }
    }

    /// Counts calls; answers with `body` or fails when `body` is empty.
    struct Counting {
        calls: AtomicUsize,
        body: Mutex<&'static str>,
        cache_control: Option<&'static str>,
    }
    impl Counting {
        fn new(body: &'static str, cache_control: Option<&'static str>) -> Self {
            Self {
                calls: AtomicUsize::new(0),
                body: Mutex::new(body),
                cache_control,
            }
        }
        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }
    impl Transport for Counting {
        async fn get(&self, _: &str) -> Result<HttpResponse, TransportError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let body = *self.body.lock().unwrap();
            if body.is_empty() {
                return Err(TransportError("down".into()));
            }
            Ok(HttpResponse {
                status: 200,
                cache_control: self.cache_control.map(str::to_owned),
                body: body.as_bytes().to_vec(),
            })
        }
    }

    const URL: &str = "https://a/manifest.json";

    fn path() -> ResourcePath {
        ResourcePath::new("stream", "movie", "tt1")
    }

    #[test]
    fn policy_table() {
        let p = CachePolicy {
            max_age: 10,
            stale_revalidate: 5,
            stale_error: 20,
        };
        let at = |s| p.freshness(Duration::from_secs(s));
        assert_eq!(at(0), Freshness::Fresh);
        assert_eq!(at(9), Freshness::Fresh);
        assert_eq!(at(10), Freshness::Stale);
        assert_eq!(at(14), Freshness::Stale);
        assert_eq!(
            at(15),
            Freshness::Expired {
                usable_on_error: true
            }
        );
        assert_eq!(
            at(30),
            Freshness::Expired {
                usable_on_error: false
            }
        );
    }

    #[test]
    fn body_hints_win_over_headers() {
        let body = CacheHints {
            cache_max_age: Some(60),
            ..Default::default()
        };
        let header = Some("max-age=5, stale-while-revalidate=1");
        assert_eq!(
            CachePolicy::from_response(body, header).unwrap().max_age,
            60
        );
        let p = CachePolicy::from_response(CacheHints::default(), header).unwrap();
        assert_eq!((p.max_age, p.stale_revalidate), (5, 1));
        assert_eq!(
            CachePolicy::from_response(CacheHints::default(), Some("public, no-cache")),
            None
        );
        assert_eq!(
            CachePolicy::from_response(CacheHints::default(), None),
            None
        );
        assert_eq!(
            CachePolicy::from_response(CacheHints::default(), Some("max-age=0")),
            None
        );
    }

    #[test]
    fn fresh_stale_expired_and_stale_on_error() {
        let clock = FakeClock::default();
        let cache = Cache::new(&clock);
        let t = Counting::new(
            r#"{"streams":[],"cacheMaxAge":10,"staleRevalidate":5,"staleError":20}"#,
            None,
        );
        let get = || block_on(cache.fetch(&t, URL, &path())).map(|(_, s)| s);

        assert_eq!(get(), Ok(Served::Network));
        clock.set(9);
        assert_eq!(get(), Ok(Served::Fresh));
        clock.set(12);
        assert_eq!(get(), Ok(Served::Stale));
        assert_eq!(t.calls(), 1);

        // Expired: fetches again (and restarts the clock for the entry).
        clock.set(16);
        assert_eq!(get(), Ok(Served::Network));
        assert_eq!(t.calls(), 2);

        // Addon goes down: within staleError the old answer is served, then not.
        *t.body.lock().unwrap() = "";
        clock.set(16 + 25);
        assert_eq!(get(), Ok(Served::StaleOnError));
        clock.set(16 + 31);
        assert!(matches!(get(), Err(Error::Transport(_))));
    }

    #[test]
    fn no_hints_means_no_cache() {
        let clock = FakeClock::default();
        let cache = Cache::new(&clock);
        let t = Counting::new(r#"{"streams":[]}"#, None);
        block_on(cache.fetch(&t, URL, &path())).unwrap();
        block_on(cache.fetch(&t, URL, &path())).unwrap();
        assert_eq!(t.calls(), 2);
    }

    #[test]
    fn header_fallback_and_revalidate() {
        let clock = FakeClock::default();
        let cache = Cache::new(&clock);
        let t = Counting::new(
            r#"{"streams":[]}"#,
            Some("max-age=10, stale-while-revalidate=10"),
        );
        block_on(cache.fetch(&t, URL, &path())).unwrap();
        clock.set(15);
        assert_eq!(
            block_on(cache.fetch(&t, URL, &path())).unwrap().1,
            Served::Stale
        );
        block_on(cache.revalidate(&t, URL, &path())).unwrap();
        assert_eq!(
            block_on(cache.fetch(&t, URL, &path())).unwrap().1,
            Served::Fresh
        );
        assert_eq!(t.calls(), 2);
    }
}
