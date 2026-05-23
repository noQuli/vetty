use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use tempfile::tempdir;
use tracing::warn;
use walkdir::WalkDir;

pub struct DiskBuilder {
    pub min_size: u64,
    pub overhead: f64,
}

impl Default for DiskBuilder {
    fn default() -> Self {
        Self {
            min_size: 16 * 1024 * 1024,
            overhead: 1.2,
        }
    }
}

impl DiskBuilder {
    pub fn build(&self, source_dir: &Path, output_path: &Path) -> Result<PathBuf> {
        if !source_dir.is_dir() {
            bail!("source directory does not exist or is not a directory: {source_dir:?}");
        }

        let size = self.calculate_size(source_dir)?;
        self.create_sparse_file(output_path, size)?;
        if let Err(error) = self.format_ext4_with_source(source_dir, output_path) {
            warn!("mkfs.ext4 direct population failed (falling back to mount+copy): {error}");
            self.format_ext4(output_path)?;
            self.copy_contents(source_dir, output_path)?;
        }
        Ok(output_path.canonicalize()?)
    }

    fn calculate_size(&self, dir: &Path) -> Result<u64> {
        let mut total: u64 = 0;
        for entry in WalkDir::new(dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                total = total.saturating_add(entry.metadata()?.len());
            }
        }

        let with_overhead = (total as f64 * self.overhead).ceil() as u64;
        let min = with_overhead.max(self.min_size);
        let mb = 1024 * 1024;
        Ok(min.div_ceil(mb) * mb)
    }

    fn create_sparse_file(&self, path: &Path, size: u64) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(path)
            .with_context(|| format!("failed to create image file at {}", path.display()))?;
        file.set_len(size)
            .with_context(|| format!("failed to set image size for {}", path.display()))?;
        Ok(())
    }

    fn format_ext4(&self, path: &Path) -> Result<()> {
        run_cmd(
            "mkfs.ext4",
            &["-F", &path.display().to_string()],
            "format ext4 image",
        )
    }

    fn format_ext4_with_source(&self, source: &Path, path: &Path) -> Result<()> {
        run_cmd(
            "mkfs.ext4",
            &[
                "-d",
                &source.display().to_string(),
                "-F",
                &path.display().to_string(),
            ],
            "format ext4 image with preloaded source directory",
        )
    }

    fn copy_contents(&self, source: &Path, image: &Path) -> Result<()> {
        let mount_dir = tempdir()?;
        let mount_path = mount_dir.path().to_path_buf();
        let mount_path_str = mount_path.display().to_string();
        let image_str = image.display().to_string();

        run_cmd(
            "sudo",
            &["mount", "-o", "loop", &image_str, &mount_path_str],
            "mount image",
        )?;

        let source_dot = format!("{}/.", source.display());
        let copy_result = run_cmd(
            "sudo",
            &["cp", "-a", &source_dot, &mount_path_str],
            "copy source directory",
        );
        let unmount_result = run_cmd("sudo", &["umount", &mount_path_str], "unmount image");

        if let Err(copy_err) = copy_result {
            if let Err(unmount_err) = unmount_result {
                bail!(
                    "copy failed: {copy_err}; additionally failed to unmount {}: {unmount_err}",
                    mount_path.display()
                );
            }
            return Err(copy_err);
        }

        unmount_result?;
        Ok(())
    }
}

fn run_cmd(program: &str, args: &[&str], context: &str) -> Result<()> {
    let output = match Command::new(program).args(args).output() {
        Ok(output) => output,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            bail!("required command `{program}` was not found on PATH while trying to {context}")
        }
        Err(err) => return Err(err).with_context(|| format!("failed to execute `{program}`")),
    };

    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    bail!(
    // ZMIANA: Dodano `{}` po programie, teraz jest `{} {}`
    "command `{} {}` failed while trying to {} (status: {})\nstdout: {}\nstderr: {}",
    program,
    args.join(" "), // warto dodać spację " ", żeby argumenty się nie zlepiły
    context,
    output.status,
    stdout,
    stderr
);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_minimum_size_for_empty_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let builder = DiskBuilder::default();
        let size = builder.calculate_size(temp.path()).expect("size");
        assert_eq!(size, 16 * 1024 * 1024);
    }
}
