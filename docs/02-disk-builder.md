# Step 2 — Disk Image Builder (`vetty-disk`)

## Goal
Create a library crate that takes a host directory and produces an ext4 disk image containing its contents, ready to attach to a Firecracker VM as a second drive.

---

## 2.1 Create the Crate

### `crates/vetty-disk/Cargo.toml`

```toml
[package]
name = "vetty-disk"
version.workspace = true
edition.workspace = true

[dependencies]
vetty-common = { path = "../vetty-common" }
anyhow = { workspace = true }
tracing = { workspace = true }
tempfile = "3"
```

---

## 2.2 Implement `crates/vetty-disk/src/lib.rs`

The builder must:

1. **Calculate the required image size**
   - Walk the source directory recursively, summing all file sizes
   - Add overhead: at least 20% extra or 16 MB minimum (ext4 metadata needs space)
   - Round up to the nearest MB

2. **Create a blank image file**
   - Use `std::fs::File::set_len()` to create a sparse file of the calculated size
   - Place the image in a temp directory or in the output path the caller provides

3. **Format the image as ext4**
   - Shell out to `mkfs.ext4 -F <image_path>`
   - Parse stderr/stdout and fail with a clear error if `mkfs.ext4` is not found

4. **Mount and copy files**
   - Create a temporary mount point directory
   - Shell out to `sudo mount -o loop <image_path> <mount_point>`
   - Use `cp -a <source_dir>/. <mount_point>/` to copy contents preserving permissions
   - `sudo umount <mount_point>`
   - Clean up the temp mount directory

5. **Return the path to the finished `.img` file**

### Public API

```rust
use std::path::{Path, PathBuf};
use anyhow::Result;

pub struct DiskBuilder {
    /// Minimum image size in bytes (default 16 MB)
    pub min_size: u64,
    /// Overhead multiplier (default 1.2 = 20%)
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
    /// Build an ext4 disk image from the given source directory.
    /// The image is written to `output_path`.
    /// Returns the canonical path to the created image.
    pub fn build(&self, source_dir: &Path, output_path: &Path) -> Result<PathBuf> {
        let size = self.calculate_size(source_dir)?;
        self.create_sparse_file(output_path, size)?;
        self.format_ext4(output_path)?;
        self.copy_contents(source_dir, output_path)?;
        Ok(output_path.canonicalize()?)
    }

    fn calculate_size(&self, dir: &Path) -> Result<u64> { todo!() }
    fn create_sparse_file(&self, path: &Path, size: u64) -> Result<()> { todo!() }
    fn format_ext4(&self, path: &Path) -> Result<()> { todo!() }
    fn copy_contents(&self, source: &Path, image: &Path) -> Result<()> { todo!() }
}
```

---

## 2.3 Implementation Details

### Calculating directory size
```rust
fn calculate_size(&self, dir: &Path) -> Result<u64> {
    let mut total: u64 = 0;
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            total += entry.metadata()?.len();
        }
    }
    let with_overhead = (total as f64 * self.overhead) as u64;
    let size = with_overhead.max(self.min_size);
    // Round up to nearest MB
    let mb = 1024 * 1024;
    Ok((size + mb - 1) / mb * mb)
}
```

> **Note:** Add `walkdir = "2"` to the dependencies.

### Shelling out safely
Use `std::process::Command` for all external calls. Always:
- Check `.status().success()`
- Capture and log stderr on failure
- Use `anyhow::bail!()` with the command and its stderr on error

### Mount/unmount pattern
```rust
fn copy_contents(&self, source: &Path, image: &Path) -> Result<()> {
    let mount_dir = tempfile::tempdir()?;
    let mount_path = mount_dir.path();

    // Mount
    run_cmd("sudo", &["mount", "-o", "loop",
        &image.display().to_string(),
        &mount_path.display().to_string()])?;

    // Copy (use a closure + defer pattern to ensure unmount)
    let copy_result = run_cmd("sudo", &["cp", "-a",
        &format!("{}/.",&source.display()),
        &mount_path.display().to_string()]);

    // Always unmount
    run_cmd("sudo", &["umount", &mount_path.display().to_string()])?;

    copy_result
}
```

---

## 2.4 Verify

```bash
cargo check -p vetty-disk
```

---

## Done Criteria

- [ ] `vetty-disk` crate compiles
- [ ] `DiskBuilder::build()` takes a source dir + output path, produces an ext4 `.img`
- [ ] Handles edge cases: empty directory, very large directory, missing `mkfs.ext4`
- [ ] Cleanup always runs (unmount even on copy failure)
