use sha2::{Digest, Sha256};
use std::fmt;

/// A complete ABI profile describing the compilation environment.
///
/// Two builds with different ABI profiles **must not** share cached artifacts,
/// as linking them would risk ODR violations or silent ABI corruption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbiProfile {
    /// Compiler family: `gcc`, `clang`, `msvc`.
    pub compiler: String,
    /// Compiler version string (e.g. `"14.1.0"`).
    pub compiler_version: String,
    /// C++ standard library: `libstdc++`, `libc++`, `msvc-stl`.
    pub stdlib: String,
    /// Target architecture: `x86_64`, `aarch64`, etc.
    pub architecture: String,
    /// Operating system: `linux`, `macos`, `windows`.
    pub os: String,
    /// Build type: `debug`, `release`, `relwithdebinfo`, `minsizerel`.
    pub build_type: String,
    /// C++ standard: `c++17`, `c++20`, etc.
    pub cxx_std: String,
    /// Whether building shared libraries.
    pub shared_libs: bool,
    /// Sorted list of enabled features (feeds into the hash — Principle 3).
    pub features: Vec<String>,
}

impl AbiProfile {
    /// Compute the SHA-256 hash of this ABI profile.
    ///
    /// This hash is used as a cache key for compiled artifacts. Two profiles
    /// with different hashes **must** produce separate build artifacts.
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();

        hasher.update(b"compiler:");
        hasher.update(self.compiler.as_bytes());
        hasher.update(b"\ncompiler_version:");
        hasher.update(self.compiler_version.as_bytes());
        hasher.update(b"\nstdlib:");
        hasher.update(self.stdlib.as_bytes());
        hasher.update(b"\narch:");
        hasher.update(self.architecture.as_bytes());
        hasher.update(b"\nos:");
        hasher.update(self.os.as_bytes());
        hasher.update(b"\nbuild_type:");
        hasher.update(self.build_type.as_bytes());
        hasher.update(b"\ncxx_std:");
        hasher.update(self.cxx_std.as_bytes());
        hasher.update(b"\nshared_libs:");
        hasher.update(if self.shared_libs {
            &b"true"[..]
        } else {
            &b"false"[..]
        });
        hasher.update(b"\nfeatures:");
        let mut sorted_features = self.features.clone();
        sorted_features.sort();
        for feat in &sorted_features {
            hasher.update(feat.as_bytes());
            hasher.update(b",");
        }

        let result = hasher.finalize();
        crate::fs::hex_encode(&result[..16])
    }

    /// Compute a header-only cache key (no compiler/arch component).
    ///
    /// Header-only packages have no compiled artifacts, so they can be
    /// shared across different compilers and architectures.
    pub fn header_only_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"header_only:");
        hasher.update(b"\nfeatures:");
        let mut sorted_features = self.features.clone();
        sorted_features.sort();
        for feat in &sorted_features {
            hasher.update(feat.as_bytes());
            hasher.update(b",");
        }
        let result = hasher.finalize();
        crate::fs::hex_encode(&result[..16])
    }
}

impl fmt::Display for AbiProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} ({}) | {} {} | {} | {}",
            self.compiler,
            self.compiler_version,
            self.stdlib,
            self.architecture,
            self.os,
            self.build_type,
            self.cxx_std
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_profile() -> AbiProfile {
        AbiProfile {
            compiler: "gcc".to_string(),
            compiler_version: "14.1.0".to_string(),
            stdlib: "libstdc++".to_string(),
            architecture: "x86_64".to_string(),
            os: "linux".to_string(),
            build_type: "debug".to_string(),
            cxx_std: "c++20".to_string(),
            shared_libs: false,
            features: vec![],
        }
    }

    #[test]
    fn test_hash_deterministic() {
        let p = sample_profile();
        assert_eq!(p.hash(), p.hash());
    }

    #[test]
    fn test_hash_changes_with_compiler() {
        let p1 = sample_profile();
        let mut p2 = sample_profile();
        p2.compiler = "clang".to_string();
        assert_ne!(p1.hash(), p2.hash());
    }

    #[test]
    fn test_hash_changes_with_features() {
        let p1 = sample_profile();
        let mut p2 = sample_profile();
        p2.features = vec!["header_only".to_string()];
        assert_ne!(p1.hash(), p2.hash());
    }

    #[test]
    fn test_features_order_independent() {
        let mut p1 = sample_profile();
        p1.features = vec!["b".to_string(), "a".to_string()];
        let mut p2 = sample_profile();
        p2.features = vec!["a".to_string(), "b".to_string()];
        assert_eq!(p1.hash(), p2.hash());
    }

    #[test]
    fn test_header_only_hash_ignores_compiler() {
        let p1 = sample_profile();
        let mut p2 = sample_profile();
        p2.compiler = "clang".to_string();
        p2.compiler_version = "18.0.0".to_string();
        p2.architecture = "aarch64".to_string();
        assert_eq!(p1.header_only_hash(), p2.header_only_hash());
    }

    #[test]
    fn test_display() {
        let p = sample_profile();
        let s = format!("{p}");
        assert!(s.contains("gcc"));
        assert!(s.contains("14.1.0"));
        assert!(s.contains("x86_64"));
    }
}
