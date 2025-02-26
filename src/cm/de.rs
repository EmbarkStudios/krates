use super::*;
use serde::de::{Deserialize, Deserializer, Error, Visitor};
type Key<'de> = std::borrow::Cow<'de, str>;

impl<'de> Deserialize<'de> for PackageId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self {
            repr: String::deserialize(deserializer)?,
        })
    }
}

macro_rules! map {
    ($kind:ty, $map:ident, $func:block) => {
        impl<'de> Deserialize<'de> for $kind {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct V;

                impl<'de> Visitor<'de> for V {
                    type Value = $kind;

                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str(concat!("an ", stringify!($kind)))
                    }

                    fn visit_map<A>(self, mut $map: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde::de::MapAccess<'de>,
                    {
                        $func
                    }
                }

                deserializer.deserialize_map(V)
            }
        }
    };
}

macro_rules! parse {
    ($kind:ty) => {
        impl<'de> Deserialize<'de> for $kind {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let s = std::borrow::Cow::<'de, str>::deserialize(deserializer)?;
                s.parse().map_err(|err| Error::custom(err))
            }
        }
    };
}

macro_rules! required {
    ($name:ident) => {
        $name.ok_or_else(|| Error::missing_field(stringify!($name)))?
    };
}

map!(Metadata, map, {
    let mut packages = None;
    let mut workspace_members = None;
    let mut workspace_default_members = None;
    let mut resolve = None;
    let mut workspace_root = None;
    let mut target_directory = None;
    let mut workspace_metadata = serde_json::Value::Null;
    let mut version = 0;

    while let Some(key) = map.next_key::<Key<'de>>()? {
        match key.as_ref() {
            "packages" => packages = Some(map.next_value()?),
            "workspace_members" => workspace_members = Some(map.next_value()?),
            "workspace_default_members" => workspace_default_members = Some(map.next_value()?),
            "resolve" => resolve = Some(map.next_value()?),
            "workspace_root" => workspace_root = Some(map.next_value()?),
            "target_directory" => target_directory = Some(map.next_value()?),
            "metadata" => workspace_metadata = map.next_value()?,
            "version" => version = map.next_value()?,
            _ => {
                map.next_value::<Ignore>()?;
            }
        }
    }

    Ok(Metadata {
        packages: required!(packages),
        workspace_members: required!(workspace_members),
        workspace_default_members: WorkspaceDefaultMembers(workspace_default_members),
        resolve,
        workspace_root: required!(workspace_root),
        target_directory: required!(target_directory),
        workspace_metadata,
        version,
    })
});

map!(Resolve, map, {
    let mut nodes = None;
    let mut root = None;

    while let Some(key) = map.next_key::<Key<'de>>()? {
        match key.as_ref() {
            "nodes" => nodes = Some(map.next_value()?),
            "root" => root = Some(map.next_value()?),
            _ => {
                map.next_value::<Ignore>()?;
            }
        }
    }

    Ok(Resolve {
        nodes: required!(nodes),
        root: required!(root),
    })
});

map!(Node, map, {
    let mut id = None;
    let mut deps = Vec::new();
    let mut dependencies = None;
    let mut features = Vec::new();

    while let Some(key) = map.next_key::<Key<'de>>()? {
        match key.as_ref() {
            "id" => id = Some(map.next_value()?),
            "deps" => deps = map.next_value()?,
            "dependencies" => dependencies = Some(map.next_value()?),
            "features" => features = map.next_value()?,
            _ => {
                map.next_value::<Ignore>()?;
            }
        }
    }

    Ok(Node {
        id: required!(id),
        deps,
        dependencies: required!(dependencies),
        features,
    })
});

map!(NodeDep, map, {
    let mut name = None;
    let mut pkg = None;
    let mut dep_kinds = Vec::new();

    while let Some(key) = map.next_key::<Key<'de>>()? {
        match key.as_ref() {
            "name" => name = Some(map.next_value()?),
            "pkg" => pkg = Some(map.next_value()?),
            "dep_kinds" => dep_kinds = map.next_value()?,
            _ => {
                map.next_value::<Ignore>()?;
            }
        }
    }

    Ok(NodeDep {
        name: required!(name),
        pkg: required!(pkg),
        dep_kinds,
    })
});

map!(DepKindInfo, map, {
    let mut kind = DependencyKind::Normal;
    let mut target = None;

    while let Some(key) = map.next_key::<Key<'de>>()? {
        match key.as_ref() {
            "kind" => kind = map.next_value()?,
            "target" => target = map.next_value()?,
            _ => {
                map.next_value::<Ignore>()?;
            }
        }
    }

    Ok(DepKindInfo { kind, target })
});

impl<'de> Deserialize<'de> for DependencyKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let kind = Option::<std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        if let Some(kind) = kind {
            Ok(match kind.as_ref() {
                "normal" => Self::Normal,
                "dev" => Self::Development,
                "build" => Self::Build,
                unknown => {
                    return Err(Error::unknown_variant(unknown, &["normal", "dev", "build"]));
                }
            })
        } else {
            Ok(Self::Normal)
        }
    }
}

map!(Dependency, map, {
    let mut name = None;
    let mut source = None;
    let mut req = None;
    let mut kind = DependencyKind::Normal;
    let mut optional = None;
    let mut uses_default_features = None;
    let mut features = None;
    let mut target = None;
    let mut rename = None;
    let mut registry = None;
    let mut path = None;

    while let Some(key) = map.next_key::<Key<'de>>()? {
        match key.as_ref() {
            "name" => name = Some(map.next_value()?),
            "source" => source = map.next_value()?,
            "req" => req = Some(map.next_value()?),
            "kind" => kind = map.next_value()?,
            "optional" => optional = Some(map.next_value()?),
            "uses_default_features" => uses_default_features = Some(map.next_value()?),
            "features" => features = Some(map.next_value()?),
            "target" => target = map.next_value()?,
            "rename" => rename = map.next_value()?,
            "registry" => registry = map.next_value()?,
            "path" => path = map.next_value()?,
            _ => {
                map.next_value::<Ignore>()?;
            }
        }
    }

    Ok(Dependency {
        name: required!(name),
        source,
        req: required!(req),
        kind,
        optional: required!(optional),
        uses_default_features: required!(uses_default_features),
        features: required!(features),
        target,
        rename,
        registry,
        path,
    })
});

parse!(TargetKind);
parse!(CrateType);
parse!(Edition);

map!(Package, map, {
    let mut name = None;
    let mut version = None;
    let mut authors = Vec::new();
    let mut id = None;
    let mut source = None;
    let mut description = None;
    let mut dependencies = None;
    let mut license = None;
    let mut license_file = None;
    let mut targets = None;
    let mut features = None;
    let mut manifest_path = None;
    let mut categories = Vec::new();
    let mut keywords = Vec::new();
    let mut readme = None;
    let mut repository = None;
    let mut homepage = None;
    let mut documentation = None;
    let mut edition = Edition::default();
    let mut metadata = serde_json::Value::Null;
    let mut links = None;
    let mut publish = None;
    let mut default_run = None;
    let mut rust_version = None;

    while let Some(key) = map.next_key::<Key<'de>>()? {
        match key.as_ref() {
            "name" => name = Some(map.next_value()?),
            "version" => version = Some(map.next_value()?),
            "authors" => authors = map.next_value()?,
            "id" => id = Some(map.next_value()?),
            "source" => source = map.next_value()?,
            "description" => description = map.next_value()?,
            "dependencies" => dependencies = Some(map.next_value()?),
            "license" => license = map.next_value()?,
            "license_file" => license_file = map.next_value()?,
            "targets" => targets = Some(map.next_value()?),
            "features" => features = Some(map.next_value()?),
            "manifest_path" => manifest_path = Some(map.next_value()?),
            "categories" => categories = map.next_value()?,
            "keywords" => keywords = map.next_value()?,
            "readme" => readme = map.next_value()?,
            "repository" => repository = map.next_value()?,
            "homepage" => homepage = map.next_value()?,
            "documentation" => documentation = map.next_value()?,
            "edition" => edition = map.next_value()?,
            "metadata" => metadata = map.next_value()?,
            "links" => links = map.next_value()?,
            "publish" => publish = map.next_value()?,
            "default_run" => default_run = map.next_value()?,
            "rust_version" => {
                let s = map.next_value::<Option<String>>()?;
                if let Some(s) = s {
                    rust_version = Some(deserialize_rust_version(s)?);
                }
            }
            _ => {
                map.next_value::<Ignore>()?;
            }
        }
    }

    Ok(Package {
        name: required!(name),
        version: required!(version),
        authors,
        id: required!(id),
        source,
        description,
        dependencies: required!(dependencies),
        license,
        license_file,
        targets: required!(targets),
        features: required!(features),
        manifest_path: required!(manifest_path),
        categories,
        keywords,
        readme,
        repository,
        homepage,
        documentation,
        edition,
        metadata,
        links,
        publish,
        default_run,
        rust_version,
    })
});

map!(Target, map, {
    let mut name = None;
    let mut kind = None;
    let mut crate_types = Vec::new();
    let mut required_features = Vec::new();
    let mut src_path = None;
    let mut edition = Edition::default();
    let mut doctest = true;
    let mut test = true;
    let mut doc = true;

    while let Some(key) = map.next_key::<Key<'de>>()? {
        match key.as_ref() {
            "name" => name = Some(map.next_value()?),
            "kind" => kind = Some(map.next_value()?),
            "crate_types" => crate_types = map.next_value()?,
            "required-features" => required_features = map.next_value()?,
            "src_path" => src_path = Some(map.next_value()?),
            "edition" => edition = map.next_value()?,
            "doctest" => doctest = map.next_value()?,
            "test" => test = map.next_value()?,
            "doc" => doc = map.next_value()?,
            _ => {
                map.next_value::<Ignore>()?;
            }
        }
    }

    Ok(Target {
        name: required!(name),
        kind: required!(kind),
        crate_types,
        required_features,
        src_path: required!(src_path),
        edition,
        doctest,
        test,
        doc,
    })
});

impl<'de> Deserialize<'de> for Source {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self {
            repr: String::deserialize(deserializer)?,
        })
    }
}

/// As per the Cargo Book the [`rust-version` field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field) must:
///
/// > be a bare version number with two or three components;
/// > it cannot include semver operators or pre-release identifiers.
///
/// [`semver::Version`] however requires three components. This function takes
/// care of appending `.0` if the provided version number only has two components
/// and ensuring that it does not contain a pre-release version or build metadata.
fn deserialize_rust_version<E: Error>(mut buf: String) -> Result<semver::Version, E> {
    for c in buf.chars() {
        if c == '-' {
            return Err(E::custom(
                "pre-release identifiers are not supported in rust-version",
            ));
        } else if c == '+' {
            return Err(E::custom("build metadata is not supported in rust-version"));
        }
    }

    if buf.matches('.').count() == 1 {
        // e.g. 1.0 -> 1.0.0
        buf.push_str(".0");
    }

    Version::parse(&buf).map_err(E::custom)
}

pub struct Ignore;

impl<'de> Visitor<'de> for Ignore {
    type Value = Self;

    fn expecting(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }

    #[inline]
    fn visit_bool<E>(self, _x: bool) -> Result<Self::Value, E> {
        Ok(Self)
    }

    #[inline]
    fn visit_i64<E>(self, _x: i64) -> Result<Self::Value, E> {
        Ok(Self)
    }

    #[inline]
    fn visit_i128<E>(self, _x: i128) -> Result<Self::Value, E> {
        Ok(Self)
    }

    #[inline]
    fn visit_u64<E>(self, _x: u64) -> Result<Self::Value, E> {
        Ok(Self)
    }

    #[inline]
    fn visit_u128<E>(self, _x: u128) -> Result<Self::Value, E> {
        Ok(Self)
    }

    #[inline]
    fn visit_f64<E>(self, _x: f64) -> Result<Self::Value, E> {
        Ok(Self)
    }

    #[inline]
    fn visit_str<E>(self, _s: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Self)
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Self)
    }

    #[inline]
    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::deserialize(deserializer)
    }

    #[inline]
    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::deserialize(deserializer)
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Self)
    }

    #[inline]
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        while let Some(Self) = seq.next_element()? {}
        Ok(Self)
    }

    #[inline]
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        while let Some((Self, Self)) = map.next_entry()? {}
        Ok(Self)
    }

    #[inline]
    fn visit_bytes<E>(self, _bytes: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Self)
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::EnumAccess<'de>,
    {
        use serde::de::VariantAccess;
        data.variant::<Self>()?.1.newtype_variant()
    }
}

impl<'de> Deserialize<'de> for Ignore {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_ignored_any(Self)
    }
}
