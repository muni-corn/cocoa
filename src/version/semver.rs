//! Semantic versioning engine.

use std::fmt;

use semver as sv;
use thiserror::Error;

/// Error from a semantic version operation.
#[derive(Debug, Error, PartialEq)]
pub enum SemVerError {
    /// The input string is not a valid semver version.
    #[error("failed to parse semver '{0}'")]
    Parse(String),
}

/// A semantic version: `MAJOR.MINOR.PATCH[-pre-release][+build-metadata]`.
///
/// Wraps the `semver` crate's `Version` type to provide bump helpers and
/// friendly error messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer(sv::Version);

impl Default for SemVer {
    fn default() -> Self {
        Self(sv::Version::new(0, 0, 0))
    }
}

impl SemVer {
    /// Parse a semver string such as `"1.2.3"` or `"1.0.0-alpha.1"`.
    pub fn parse(s: &str) -> Result<Self, SemVerError> {
        sv::Version::parse(s)
            .map(SemVer)
            .map_err(|_| SemVerError::Parse(s.to_string()))
    }

    /// Return the major version component.
    pub fn major(&self) -> u64 {
        self.0.major
    }

    /// Return the minor version component.
    pub fn minor(&self) -> u64 {
        self.0.minor
    }

    /// Return the patch version component.
    pub fn patch(&self) -> u64 {
        self.0.patch
    }

    /// Return the pre-release identifier, or an empty string if absent.
    pub fn pre_release(&self) -> &str {
        self.0.pre.as_str()
    }

    /// Return the build-metadata identifier, or an empty string if absent.
    pub fn build_metadata(&self) -> &str {
        self.0.build.as_str()
    }

    /// Bump the major version, resetting minor, patch, and pre-release.
    ///
    /// Build metadata is also cleared per the SemVer spec.
    pub fn bump_major(&self) -> Self {
        let mut v = self.0.clone();
        // in major version 0, only bump the minor version for breaking changes
        if self.major() == 0 {
            v.minor += 1;
            v.patch = 0;
            v.pre = sv::Prerelease::EMPTY;
            v.build = sv::BuildMetadata::EMPTY;
        } else {
            v.major += 1;
            v.minor = 0;
            v.patch = 0;
            v.pre = sv::Prerelease::EMPTY;
            v.build = sv::BuildMetadata::EMPTY;
        }
        SemVer(v)
    }

    /// Bump the minor version, resetting patch and pre-release.
    pub fn bump_minor(&self) -> Self {
        let mut v = self.0.clone();
        // in major version 0, only bump the patch version for minor releases
        if self.major() == 0 {
            v.patch += 1;
            v.pre = sv::Prerelease::EMPTY;
            v.build = sv::BuildMetadata::EMPTY;
        } else {
            v.minor += 1;
            v.patch = 0;
            v.pre = sv::Prerelease::EMPTY;
            v.build = sv::BuildMetadata::EMPTY;
        }
        SemVer(v)
    }

    /// Bump the patch version, clearing pre-release and build metadata.
    pub fn bump_patch(&self) -> Self {
        let mut v = self.0.clone();
        v.patch += 1;
        v.pre = sv::Prerelease::EMPTY;
        v.build = sv::BuildMetadata::EMPTY;
        SemVer(v)
    }

    /// Return a copy with the given pre-release identifier (e.g. `"alpha.1"`).
    pub fn with_pre_release(mut self, pre: &str) -> Result<Self, SemVerError> {
        self.0.pre = sv::Prerelease::new(pre).map_err(|_| SemVerError::Parse(pre.to_string()))?;
        Ok(self)
    }

    /// Return a copy with the given build-metadata identifier (e.g.
    /// `"20240316"`).
    pub fn with_build_metadata(mut self, meta: &str) -> Result<Self, SemVerError> {
        self.0.build =
            sv::BuildMetadata::new(meta).map_err(|_| SemVerError::Parse(meta.to_string()))?;
        Ok(self)
    }

    /// Return a reference to the inner `semver::Version`.
    pub fn inner(&self) -> &sv::Version {
        &self.0
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let v = SemVer::parse("1.2.3").unwrap();
        assert_eq!(v.major(), 1);
        assert_eq!(v.minor(), 2);
        assert_eq!(v.patch(), 3);
        assert_eq!(v.pre_release(), "");
        assert_eq!(v.build_metadata(), "");
    }

    #[test]
    fn test_parse_with_pre_release() {
        let v = SemVer::parse("1.0.0-alpha.1").unwrap();
        assert_eq!(v.pre_release(), "alpha.1");
    }

    #[test]
    fn test_parse_with_build_metadata() {
        let v = SemVer::parse("1.0.0+20240316").unwrap();
        assert_eq!(v.build_metadata(), "20240316");
    }

    #[test]
    fn test_parse_invalid() {
        assert!(SemVer::parse("not-a-version").is_err());
        assert!(SemVer::parse("1.2").is_err());
        assert!(SemVer::parse("").is_err());
    }

    #[test]
    fn test_bump_major() {
        let v = SemVer::parse("1.2.3").unwrap();
        let bumped = v.bump_major();
        assert_eq!(bumped.to_string(), "2.0.0");
    }

    #[test]
    fn test_bump_major_clears_pre_release() {
        let v = SemVer::parse("1.2.3-beta.1").unwrap();
        let bumped = v.bump_major();
        assert_eq!(bumped.to_string(), "2.0.0");
        assert_eq!(bumped.pre_release(), "");
    }

    #[test]
    fn test_bump_minor() {
        let v = SemVer::parse("1.2.3").unwrap();
        let bumped = v.bump_minor();
        assert_eq!(bumped.to_string(), "1.3.0");
    }

    #[test]
    fn test_bump_patch() {
        let v = SemVer::parse("1.2.3").unwrap();
        let bumped = v.bump_patch();
        assert_eq!(bumped.to_string(), "1.2.4");
    }

    #[test]
    fn test_with_pre_release() {
        let v = SemVer::parse("1.0.0").unwrap();
        let pre = v.with_pre_release("rc.1").unwrap();
        assert_eq!(pre.to_string(), "1.0.0-rc.1");
    }

    #[test]
    fn test_with_build_metadata() {
        let v = SemVer::parse("1.0.0").unwrap();
        let meta = v.with_build_metadata("build.42").unwrap();
        assert_eq!(meta.to_string(), "1.0.0+build.42");
    }

    #[test]
    fn test_ordering() {
        let v1 = SemVer::parse("1.0.0").unwrap();
        let v2 = SemVer::parse("1.0.1").unwrap();
        let v3 = SemVer::parse("2.0.0").unwrap();
        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
    }

    #[test]
    fn test_display() {
        assert_eq!(SemVer::parse("0.1.0").unwrap().to_string(), "0.1.0");
        assert_eq!(
            SemVer::parse("3.14.159-alpha+build").unwrap().to_string(),
            "3.14.159-alpha+build"
        );
    }
}
