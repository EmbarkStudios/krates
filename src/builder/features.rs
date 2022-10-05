#[derive(Debug, PartialEq, Eq)]
pub enum Feature<'feat> {
    Krate(&'feat str),
    Weak {
        krate: &'feat str,
        feature: &'feat str,
    },
    Strong {
        krate: &'feat str,
        feature: &'feat str,
    },
    Simple(&'feat str),
}

enum FeatureKind {
    Krate,
    Weak(usize),
    Strong(usize),
    Simple,
}

pub struct ParsedFeature<'feat> {
    inner: &'feat str,
    kind: FeatureKind,
}

impl<'feat> ParsedFeature<'feat> {
    #[inline]
    pub fn feat(&self) -> Feature<'feat> {
        match self.kind {
            FeatureKind::Krate => Feature::Krate(&self.inner[4..]),
            FeatureKind::Weak(ind) => Feature::Weak {
                krate: &self.inner[..ind],
                feature: &self.inner[ind + 2..],
            },
            FeatureKind::Strong(ind) => Feature::Strong {
                krate: &self.inner[..ind],
                feature: &self.inner[ind + 1..],
            },
            FeatureKind::Simple => Feature::Simple(self.inner),
        }
    }
}

impl<'feat> From<&'feat str> for ParsedFeature<'feat> {
    #[inline]
    fn from(f: &'feat str) -> Self {
        let kind = if f.starts_with("dep:") {
            FeatureKind::Krate
        } else if let Some(ind) = f.find("?/") {
            FeatureKind::Weak(ind)
        } else if let Some(ind) = f.find('/') {
            FeatureKind::Strong(ind)
        } else {
            FeatureKind::Simple
        };

        Self { inner: f, kind }
    }
}

#[cfg(test)]
mod test {
    use super::{Feature, ParsedFeature};
    use similar_asserts::assert_eq;

    #[test]
    fn parses_features() {
        assert_eq!(
            ParsedFeature::from("simple").feat(),
            Feature::Simple("simple"),
        );

        assert_eq!(
            ParsedFeature::from("dep:krate-name-here").feat(),
            Feature::Krate("krate-name-here"),
        );

        assert_eq!(
            ParsedFeature::from("krate-name-here?/feature-name-here").feat(),
            Feature::Weak {
                krate: "krate-name-here",
                feature: "feature-name-here"
            },
        );

        assert_eq!(
            ParsedFeature::from("name/feat").feat(),
            Feature::Strong {
                krate: "name",
                feature: "feat"
            },
        );
    }
}
