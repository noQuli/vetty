use std::path::PathBuf;

use vetty_common::GUEST_CID;

#[derive(Debug, Clone)]
pub struct VmConfig {
    pub firecracker_bin: PathBuf,
    pub kernel_path: PathBuf,
    pub rootfs_path: PathBuf,
    pub code_disk_path: PathBuf,
    pub vcpu_count: u8,
    pub mem_size_mb: u32,
    pub guest_cid: u32,
    pub vsock_uds_path: PathBuf,
    pub boot_args: String,
    pub tap_device: String,
    pub guest_mac: String,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            firecracker_bin: PathBuf::from("firecracker"),
            kernel_path: PathBuf::new(),
            rootfs_path: PathBuf::new(),
            code_disk_path: PathBuf::new(),
            vcpu_count: 1,
            mem_size_mb: 128,
            guest_cid: GUEST_CID,
            vsock_uds_path: PathBuf::from("/tmp/vetty_v.sock"),
            boot_args: "console=ttyS0 reboot=k panic=1".to_string(),
            tap_device: "tap0".to_string(),
            guest_mac: "06:00:AC:10:00:02".to_string(),
        }
    }
}
