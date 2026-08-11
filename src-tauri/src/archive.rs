//! Shared archive extraction for downloaded model and sidecar artifacts.
//!
//! This module is the single owner for unpacking `.zip`, `.tar.gz`, and
//! `.tar.bz2` downloads so every acquisition path (models, sidecar binaries)
//! behaves identically. Both "flat" archives (entries at the archive root,
//! e.g. llama.cpp Windows releases) and "wrapped" archives (a single
//! top-level directory, e.g. llama.cpp Ubuntu releases) are handled
//! transparently.

use anyhow::{anyhow, bail, Context};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

/// How extracted entries are laid out inside the target directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractLayout {
    /// Keep the archive's directory structure. When every entry shares a
    /// single top-level directory (a "wrapped" archive), that wrapper is
    /// stripped so files land directly in the target directory.
    PreservePaths,
    /// Discard directory structure entirely; every file lands directly in the
    /// target directory under its file name.
    Flat,
}

/// Extracts `archive_path` into `target_dir`. The format is detected from the
/// file extension: `.zip`, `.tar.gz` / `.tgz`, or `.tar.bz2` / `.tbz2`.
pub fn extract_archive(
    archive_path: &Path,
    target_dir: &Path,
    layout: ExtractLayout,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(target_dir)
        .with_context(|| format!("Failed to create target dir {}", target_dir.display()))?;

    let archive_name = archive_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("Invalid archive path: {}", archive_path.display()))?;

    if archive_name.ends_with(".zip") {
        extract_zip(archive_path, target_dir, layout)
    } else if archive_name.ends_with(".tar.gz") || archive_name.ends_with(".tgz") {
        let decoder = flate2::read::GzDecoder::new(open_archive(archive_path)?);
        extract_tar(decoder, target_dir, layout)
    } else if archive_name.ends_with(".tar.bz2") || archive_name.ends_with(".tbz2") {
        let decoder = bzip2::read::BzDecoder::new(open_archive(archive_path)?);
        extract_tar(decoder, target_dir, layout)
    } else {
        bail!("Unsupported archive format: {}", archive_name)
    }
}

fn open_archive(archive_path: &Path) -> anyhow::Result<std::fs::File> {
    std::fs::File::open(archive_path)
        .with_context(|| format!("Failed to open archive {}", archive_path.display()))
}

fn extract_zip(
    archive_path: &Path,
    target_dir: &Path,
    layout: ExtractLayout,
) -> anyhow::Result<()> {
    let mut archive = zip::ZipArchive::new(open_archive(archive_path)?)
        .with_context(|| format!("Failed to open zip archive {}", archive_path.display()))?;

    let mut roots = BTreeSet::new();
    let mut all_wrapped = true;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .context("Failed to read zip entry")?;
        let Some(enclosed) = entry.enclosed_name() else {
            continue;
        };
        let Some(relative) = sanitize_relative_path(&enclosed) else {
            continue;
        };
        if layout == ExtractLayout::Flat && entry.is_dir() {
            continue;
        }
        let Some(out_path) =
            output_path(target_dir, &relative, layout, &mut roots, &mut all_wrapped)
        else {
            continue;
        };

        if entry.is_dir() {
            let _ = std::fs::create_dir_all(&out_path);
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create dir {}", parent.display()))?;
        }
        let mut out_file = std::fs::File::create(&out_path)
            .with_context(|| format!("Failed to create file {}", out_path.display()))?;
        std::io::copy(&mut entry, &mut out_file)
            .with_context(|| format!("Failed to extract file {}", out_path.display()))?;
    }

    flatten_single_root(target_dir, &roots, all_wrapped, layout)
}

fn extract_tar<R: std::io::Read>(
    reader: R,
    target_dir: &Path,
    layout: ExtractLayout,
) -> anyhow::Result<()> {
    let mut archive = tar::Archive::new(reader);

    let mut roots = BTreeSet::new();
    let mut all_wrapped = true;

    for entry in archive.entries().context("Failed to read tar entries")? {
        let mut entry = entry.context("Failed to read tar entry")?;
        let raw_path = entry.path().context("Invalid path in tar")?.into_owned();
        let Some(relative) = sanitize_relative_path(&raw_path) else {
            continue;
        };
        if layout == ExtractLayout::Flat && entry.header().entry_type().is_dir() {
            continue;
        }
        let Some(out_path) =
            output_path(target_dir, &relative, layout, &mut roots, &mut all_wrapped)
        else {
            continue;
        };

        if entry.header().entry_type().is_symlink() {
            #[cfg(unix)]
            {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("Failed to create dir {}", parent.display()))?;
                }
                let target = entry
                    .link_name()
                    .context("Invalid symlink target")?
                    .ok_or_else(|| anyhow!("Symlink with no target"))?;
                std::os::unix::fs::symlink(&target, &out_path)
                    .with_context(|| format!("Failed to create symlink {}", out_path.display()))?;
            }
        } else if entry.header().entry_type().is_dir() {
            let _ = std::fs::create_dir_all(&out_path);
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create dir {}", parent.display()))?;
            }
            let mut out_file = std::fs::File::create(&out_path)
                .with_context(|| format!("Failed to create file {}", out_path.display()))?;
            std::io::copy(&mut entry, &mut out_file)
                .with_context(|| format!("Failed to extract file {}", out_path.display()))?;
        }
    }

    flatten_single_root(target_dir, &roots, all_wrapped, layout)
}

/// Keeps only `Normal` path components, stripping any root, prefix, or `..`
/// traversal so entries can never escape the target directory. Returns `None`
/// when nothing usable remains.
fn sanitize_relative_path(path: &Path) -> Option<PathBuf> {
    let sanitized: PathBuf = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part),
            _ => None,
        })
        .collect();
    if sanitized.as_os_str().is_empty() {
        None
    } else {
        Some(sanitized)
    }
}

/// Computes where an entry lands for the given layout, tracking whether the
/// archive is "wrapped" (every entry shares one top-level directory) so the
/// wrapper can be stripped afterwards.
fn output_path(
    target_dir: &Path,
    relative: &Path,
    layout: ExtractLayout,
    roots: &mut BTreeSet<OsString>,
    all_wrapped: &mut bool,
) -> Option<PathBuf> {
    match layout {
        ExtractLayout::Flat => relative
            .file_name()
            .map(|file_name| target_dir.join(file_name)),
        ExtractLayout::PreservePaths => {
            let mut components = relative.components();
            match (components.next(), components.next()) {
                (Some(Component::Normal(root)), Some(_)) => {
                    roots.insert(root.to_os_string());
                }
                _ => *all_wrapped = false,
            }
            Some(target_dir.join(relative))
        }
    }
}

/// The single shared top-level directory of a wrapped archive, if any.
fn shared_root(roots: &BTreeSet<OsString>, all_wrapped: bool) -> Option<&OsString> {
    if all_wrapped && roots.len() == 1 {
        roots.iter().next()
    } else {
        None
    }
}

/// For `PreservePaths` extractions that turned out to be wrapped, moves the
/// wrapper directory's contents up into the target directory. Renames stay on
/// the same filesystem, so this is cheap even for large models.
fn flatten_single_root(
    target_dir: &Path,
    roots: &BTreeSet<OsString>,
    all_wrapped: bool,
    layout: ExtractLayout,
) -> anyhow::Result<()> {
    if layout != ExtractLayout::PreservePaths {
        return Ok(());
    }
    let Some(root_name) = shared_root(roots, all_wrapped) else {
        return Ok(());
    };
    let root_dir = target_dir.join(root_name);
    if !root_dir.is_dir() {
        return Ok(());
    }

    for child in std::fs::read_dir(&root_dir)
        .with_context(|| format!("Failed to list dir {}", root_dir.display()))?
    {
        let child = child.context("Failed to read dir entry")?;
        let destination = target_dir.join(child.file_name());
        if destination.exists() {
            bail!(
                "Cannot flatten archive: {} already exists",
                destination.display()
            );
        }
        std::fs::rename(child.path(), &destination)
            .with_context(|| format!("Failed to move {} into place", destination.display()))?;
    }
    std::fs::remove_dir(&root_dir)
        .with_context(|| format!("Failed to remove wrapper dir {}", root_dir.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "voquill-archive-test-{}-{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn shared_root_detects_wrapped_archive() {
        let roots: BTreeSet<OsString> = [OsString::from("llama-b10331")].into_iter().collect();
        assert_eq!(
            shared_root(&roots, true),
            Some(&OsString::from("llama-b10331"))
        );
    }

    #[test]
    fn shared_root_rejects_flat_archive() {
        // A root-level file sets all_wrapped = false.
        let roots: BTreeSet<OsString> = BTreeSet::new();
        assert_eq!(shared_root(&roots, false), None);
    }

    #[test]
    fn shared_root_rejects_multiple_roots() {
        let roots: BTreeSet<OsString> = [OsString::from("a"), OsString::from("b")]
            .into_iter()
            .collect();
        assert_eq!(shared_root(&roots, true), None);
    }

    #[test]
    fn sanitize_strips_traversal_and_prefixes() {
        // `..` segments are dropped (never resolved), so the result can never
        // escape the target directory even though "evil" survives as a normal
        // directory name.
        let path = Path::new("/etc/../evil/../../good.txt");
        assert_eq!(
            sanitize_relative_path(path),
            Some(PathBuf::from("etc").join("evil").join("good.txt"))
        );
        assert_eq!(sanitize_relative_path(Path::new("..")), None);
    }

    /// Regression test: llama.cpp Windows releases ship entries at the zip
    /// root with no wrapping directory; they must land flat in the target.
    #[test]
    fn flat_zip_extracts_root_level_files() {
        let dir = temp_test_dir("flat-zip");
        let archive_path = dir.join("flat.zip");
        let target_dir = dir.join("out");

        {
            let file = std::fs::File::create(&archive_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            writer.start_file("llama-server.exe", options).unwrap();
            writer.write_all(b"binary").unwrap();
            writer.start_file("ggml.dll", options).unwrap();
            writer.write_all(b"dll").unwrap();
            writer.finish().unwrap();
        }

        extract_archive(&archive_path, &target_dir, ExtractLayout::PreservePaths).unwrap();
        assert_eq!(
            std::fs::read(target_dir.join("llama-server.exe")).unwrap(),
            b"binary",
        );
        assert_eq!(std::fs::read(target_dir.join("ggml.dll")).unwrap(), b"dll",);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Wrapped archives (e.g. llama.cpp Ubuntu tarballs) must have their
    /// single top-level directory stripped.
    #[test]
    fn wrapped_tar_strips_single_root() {
        let dir = temp_test_dir("wrapped-tar");
        let archive_path = dir.join("wrapped.tar");
        let target_dir = dir.join("out");
        std::fs::create_dir_all(&target_dir).unwrap();

        {
            let file = std::fs::File::create(&archive_path).unwrap();
            let mut builder = tar::Builder::new(file);
            let mut header = tar::Header::new_gnu();
            header.set_size(6);
            header.set_cksum();
            builder
                .append_data(&mut header, "llama-b10331/bin/tool", b"binary".as_slice())
                .unwrap();
            builder.finish().unwrap();
        }

        let reader = std::fs::File::open(&archive_path).unwrap();
        extract_tar(reader, &target_dir, ExtractLayout::PreservePaths).unwrap();
        assert_eq!(
            std::fs::read(target_dir.join("bin/tool")).unwrap(),
            b"binary",
        );
        assert!(!target_dir.join("llama-b10331").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Flat layout discards directory structure entirely (sherpa-onnx sidecar
    /// archives extract every file to the bin dir by file name).
    #[test]
    fn flat_layout_discards_directories() {
        let dir = temp_test_dir("flat-layout");
        let archive_path = dir.join("nested.tar");
        let target_dir = dir.join("out");
        std::fs::create_dir_all(&target_dir).unwrap();

        {
            let file = std::fs::File::create(&archive_path).unwrap();
            let mut builder = tar::Builder::new(file);
            let mut header = tar::Header::new_gnu();
            header.set_size(4);
            header.set_cksum();
            builder
                .append_data(&mut header, "pkg/bin/tool", b"tool".as_slice())
                .unwrap();
            let mut header = tar::Header::new_gnu();
            header.set_size(3);
            header.set_cksum();
            builder
                .append_data(&mut header, "pkg/lib/libx.so", b"lib".as_slice())
                .unwrap();
            builder.finish().unwrap();
        }

        let reader = std::fs::File::open(&archive_path).unwrap();
        extract_tar(reader, &target_dir, ExtractLayout::Flat).unwrap();
        assert_eq!(std::fs::read(target_dir.join("tool")).unwrap(), b"tool");
        assert_eq!(std::fs::read(target_dir.join("libx.so")).unwrap(), b"lib");
        assert!(!target_dir.join("pkg").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
