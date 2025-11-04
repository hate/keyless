use candle_core::{DType, Device, Tensor};
use tracing::info;

/// Cached device instance (selected once per process, reused for all operations).
static DEVICE_CACHE: std::sync::OnceLock<Device> = std::sync::OnceLock::new();

/// Probe and cache the preferred Candle device (Metal > CUDA > CPU) and perform a tiny warm-up.
///
/// Safe to call multiple times; the device is selected only once per process.
pub fn preload_device() -> Device {
    // Check cache first (device selection is expensive; avoid repeated GPU probes).
    if let Some(d) = DEVICE_CACHE.get() {
        return d.clone();
    }
    // Fallback chain: Metal (macOS) → CUDA (Linux/Windows) → CPU (always available).
    let device = match Device::new_metal(0) {
        Ok(d) => {
            info!("using Metal GPU device");
            d
        }
        Err(_) => match Device::new_cuda(0) {
            Ok(d) => {
                info!("using CUDA GPU device");
                d
            }
            // CPU fallback (slower but always works).
            Err(_) => {
                info!("using CPU device");
                Device::Cpu
            }
        },
    };
    // Best-effort kernel warm-up (first tensor op often slower; this primes the driver).
    // Ignore errors (warm-up failure doesn't prevent inference).
    let _ = Tensor::zeros((1,), DType::F32, &device);
    // Cache device for reuse (OnceLock ensures single initialization).
    let _ = DEVICE_CACHE.set(device.clone());
    device
}
