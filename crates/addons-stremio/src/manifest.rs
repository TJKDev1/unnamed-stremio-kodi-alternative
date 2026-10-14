//! Manifest fetch and validation (spec: Manifest validation).

use crate::model::{Manifest, ManifestResource, parse};
use crate::transport::Transport;
use crate::{Error, MANIFEST_PATH};

/// Something odd about a manifest that doesn't stop us using it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning(pub String);

/// A manifest that passed validation, with what looked odd about it.
#[derive(Debug, Clone, PartialEq)]
pub struct Validated {
    pub manifest: Manifest,
    pub warnings: Vec<Warning>,
}

impl Manifest {
    /// Parses and validates a manifest body.
    pub fn from_json(bytes: &[u8]) -> Result<Validated, Error> {
        let parsed = parse::<Manifest>(bytes).map_err(|e| Error::Manifest(e.0))?;
        let manifest = parsed.value;
        for (field, value) in [
            ("id", &manifest.id),
            ("name", &manifest.name),
            ("version", &manifest.version),
        ] {
            if value.trim().is_empty() {
                return Err(Error::Manifest(format!("`{field}` is empty")));
            }
        }
        let mut warnings: Vec<Warning> = parsed
            .dropped
            .into_iter()
            .map(|d| Warning(format!("dropped {}: {}", d.item, d.error)))
            .collect();
        let mut warn = |msg: String| warnings.push(Warning(msg));
        // Stremio accepts an empty list (an unconfigured AIOStreams serves one); the addon
        // just never matches a request.
        if manifest.resources.is_empty() {
            warn("no usable `resources`: the addon serves nothing".into());
        }
        let known_type = |t: &String| manifest.types.contains(t);
        for resource in &manifest.resources {
            for t in resource.types().unwrap_or_default() {
                if !known_type(t) {
                    warn(format!(
                        "resource `{}` uses type `{t}` not in `types`",
                        resource.name()
                    ));
                }
            }
            if let ManifestResource::Full(full) = resource
                && full.types.is_none()
            {
                warn(format!(
                    "resource object `{}` has no `types`: it matches nothing",
                    full.name
                ));
            }
        }
        for catalog in &manifest.catalogs {
            if !known_type(&catalog.r#type) {
                warn(format!(
                    "catalog `{}` uses type `{}` not in `types`",
                    catalog.id, catalog.r#type
                ));
            }
            if catalog.legacy_extra {
                warn(format!(
                    "catalog `{}` uses legacy `extraSupported`/`extraRequired`",
                    catalog.id
                ));
            }
        }
        if !manifest.catalogs.is_empty()
            && !manifest.resources.iter().any(|r| r.name() == "catalog")
        {
            warn("has `catalogs` but no `catalog` resource".into());
        }
        Ok(Validated { manifest, warnings })
    }
}

/// Checks an addon transport URL and returns it normalized (`stremio://` → `https://`).
pub fn transport_url(url: &str) -> Result<String, Error> {
    let url = url.trim();
    let url = match url.strip_prefix("stremio://") {
        Some(rest) => format!("https://{rest}"),
        None => url.to_owned(),
    };
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        let scheme = url.split(':').next().unwrap_or_default();
        return Err(Error::Unsupported(format!("transport `{scheme}`")));
    }
    let path = url.split(['?', '#']).next().unwrap_or(&url);
    if path.trim_end_matches('/').ends_with("/stremio/v1") {
        return Err(Error::Unsupported("legacy /stremio/v1 transport".into()));
    }
    if !path.ends_with(MANIFEST_PATH) {
        return Err(Error::InvalidUrl(format!(
            "addon URL must end with {MANIFEST_PATH}"
        )));
    }
    Ok(url)
}

/// Fetches and validates the manifest at `url`.
pub async fn fetch_manifest(transport: &impl Transport, url: &str) -> Result<Validated, Error> {
    let url = transport_url(url)?;
    let response = transport.get(&url).await.map_err(Error::Transport)?;
    if !(200..300).contains(&response.status) {
        return Err(Error::Status(response.status));
    }
    Manifest::from_json(&response.body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn validate(v: serde_json::Value) -> Result<Validated, Error> {
        Manifest::from_json(&serde_json::to_vec(&v).unwrap())
    }

    fn base() -> serde_json::Value {
        json!({ "id": "a", "name": "A", "version": "1.0.0", "resources": ["stream"], "types": ["movie"] })
    }

    #[test]
    fn minimal_manifest_is_valid() {
        let v = validate(base()).unwrap();
        assert!(v.warnings.is_empty(), "{:?}", v.warnings);
        assert_eq!(v.manifest.resources[0].name(), "stream");
        assert!(v.manifest.catalogs.is_empty());
    }

    #[test]
    fn required_fields() {
        for field in ["id", "name", "version", "resources"] {
            let mut m = base();
            m.as_object_mut().unwrap().remove(field);
            assert!(validate(m).is_err(), "missing {field}");
        }
        let mut m = base();
        m["id"] = json!(" ");
        assert!(validate(m).is_err());
    }

    #[test]
    fn empty_resources_load_with_a_warning() {
        for resources in [json!([]), json!([7])] {
            let mut m = base();
            m["resources"] = resources;
            let v = validate(m).unwrap();
            assert!(v.manifest.resources.is_empty());
            assert!(!v.warnings.is_empty());
        }
    }

    #[test]
    fn resource_objects_and_strings() {
        let mut m = base();
        m["types"] = json!(["movie", "series"]);
        m["resources"] =
            json!(["catalog", { "name": "meta", "types": ["series"], "idPrefixes": ["kitsu:"] }]);
        let v = validate(m).unwrap().manifest;
        assert_eq!(v.resources[1].name(), "meta");
        assert_eq!(
            v.resources[1].id_prefixes(),
            Some(&["kitsu:".to_owned()][..])
        );
        assert_eq!(v.resources[0].types(), None);
    }

    #[test]
    fn legacy_extras_are_normalized() {
        let mut m = base();
        m["resources"] = json!(["catalog"]);
        m["catalogs"] = json!([{ "type": "movie", "id": "top",
            "extraSupported": ["search", "genre"], "extraRequired": ["search"] }]);
        let v = validate(m).unwrap();
        let c = &v.manifest.catalogs[0];
        assert!(c.legacy_extra);
        assert_eq!(c.extra.len(), 2);
        assert!(c.extra[0].is_required && c.extra[0].name == "search");
        assert!(!c.extra[1].is_required);
        assert!(c.is_extra_supported(&["search"]));
        assert!(!c.is_extra_supported(&["genre"]));
        assert!(!c.is_extra_supported(&["search", "skip"]));
        assert_eq!(v.warnings.len(), 1);
        let out = serde_json::to_value(&v.manifest).unwrap();
        assert!(out["catalogs"][0].get("extraSupported").is_none());
        assert_eq!(out["catalogs"][0]["extra"][0]["isRequired"], true);
    }

    #[test]
    fn legacy_required_only_is_still_supported() {
        let mut m = base();
        m["resources"] = json!(["catalog"]);
        m["catalogs"] = json!([{ "type": "movie", "id": "s", "extraRequired": ["search"] }]);
        let c = &validate(m).unwrap().manifest.catalogs[0];
        assert!(c.is_extra_supported(&["search"]));
    }

    #[test]
    fn modern_extras() {
        let mut m = base();
        m["resources"] = json!(["catalog"]);
        m["catalogs"] = json!([{ "type": "movie", "id": "top", "extra": [
            { "name": "genre", "options": ["Action", 1], "optionsLimit": "2" }, { "name": "skip" }
        ]}]);
        let v = validate(m).unwrap();
        let c = &v.manifest.catalogs[0];
        assert!(!c.legacy_extra);
        assert_eq!(
            c.extra[0].options.as_deref(),
            Some(&["Action".to_owned(), "1".to_owned()][..])
        );
        assert_eq!(c.extra[0].options_limit, Some(2));
        assert!(c.is_extra_supported(&[]));
        assert!(c.is_extra_supported(&["genre", "skip"]));
    }

    #[test]
    fn warnings() {
        let mut m = base();
        m["resources"] = json!([{ "name": "meta", "types": ["tv"] }, { "name": "stream" }]);
        m["catalogs"] = json!([{ "type": "anime", "id": "x" }]);
        let w = validate(m).unwrap().warnings;
        // type `tv` unknown, resource object without types, catalog type `anime` unknown,
        // catalogs without a catalog resource.
        assert_eq!(w.len(), 4, "{w:?}");
    }

    #[test]
    fn transport_urls() {
        assert_eq!(
            transport_url("stremio://a.b/x/manifest.json").unwrap(),
            "https://a.b/x/manifest.json"
        );
        assert!(transport_url("https://a.b/manifest.json?x=1").is_ok());
        assert!(matches!(
            transport_url("https://a.b/stremio/v1"),
            Err(Error::Unsupported(_))
        ));
        assert!(matches!(
            transport_url("ipfs://abc/manifest.json"),
            Err(Error::Unsupported(_))
        ));
        assert!(matches!(
            transport_url("https://a.b/"),
            Err(Error::InvalidUrl(_))
        ));
    }
}
