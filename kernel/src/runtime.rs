//! Runtime Manager — executable-format resolution before process creation.

use crate::personality::PersonalityId;

#[derive(Clone, Copy, PartialEq)]
pub enum ExecutableFormat { Elf64, Pe, ApkContainer, Unknown }

#[derive(Clone, Copy, PartialEq)]
pub enum LaunchKind { NativeAether, Linux, Windows, Android, Unsupported }

pub fn detect_format(image: &[u8]) -> ExecutableFormat {
    if image.len() >= 20 && image[0] == 0x7f && image[1] == b'E' && image[2] == b'L' && image[3] == b'F'
        && image[4] == 2 && image[5] == 1 && image[18] == 0x3e && image[19] == 0 {
        return ExecutableFormat::Elf64;
    }
    if image.len() >= 2 && image[0] == b'M' && image[1] == b'Z' { return ExecutableFormat::Pe; }
    if image.len() >= 4 && image[0] == b'P' && image[1] == b'K' && image[2] == 3 && image[3] == 4 {
        return ExecutableFormat::ApkContainer;
    }
    ExecutableFormat::Unknown
}

/// Resolve an external launch request. An explicit kind is authoritative for
/// trusted/native callers; otherwise the executable format selects a Personality.
pub fn resolve(image: &[u8], requested: Option<LaunchKind>) -> LaunchKind {
    if let Some(kind) = requested {
        return match (kind, detect_format(image)) {
            (LaunchKind::NativeAether, ExecutableFormat::Elf64) => LaunchKind::NativeAether,
            (LaunchKind::Linux, ExecutableFormat::Elf64) => LaunchKind::Linux,
            (LaunchKind::Windows, ExecutableFormat::Pe) => LaunchKind::Windows,
            (LaunchKind::Android, ExecutableFormat::ApkContainer) => LaunchKind::Android,
            _ => LaunchKind::Unsupported,
        };
    }
    match detect_format(image) {
        ExecutableFormat::Elf64 => LaunchKind::Linux,
        ExecutableFormat::Pe => LaunchKind::Windows,
        ExecutableFormat::ApkContainer => LaunchKind::Android,
        ExecutableFormat::Unknown => LaunchKind::Unsupported,
    }
}

pub fn personality_for(kind: LaunchKind) -> PersonalityId {
    match kind {
        LaunchKind::NativeAether => PersonalityId::None,
        LaunchKind::Linux => PersonalityId::Linux,
        LaunchKind::Windows => PersonalityId::Windows,
        LaunchKind::Android => PersonalityId::Android,
        LaunchKind::Unsupported => PersonalityId::None,
    }
}

pub fn is_compatible(kind: LaunchKind) -> bool { kind != LaunchKind::Unsupported }
