//! Asset access that works both on desktop and inside an Android APK.

#[cfg(target_os = "android")]
use std::sync::OnceLock;

#[cfg(target_os = "android")]
static ANDROID_ASSETS: OnceLock<ndk::asset::AssetManager> = OnceLock::new();

#[cfg(target_os = "android")]
pub fn set_android_assets(mgr: ndk::asset::AssetManager) {
    let _ = ANDROID_ASSETS.set(mgr);
}

/// Read a packaged asset by its path below `assets/`.
pub fn load(name: &str) -> anyhow::Result<Vec<u8>> {
    #[cfg(target_os = "android")]
    {
        use std::io::Read;
        let mgr = ANDROID_ASSETS
            .get()
            .ok_or_else(|| anyhow::anyhow!("android asset manager not initialised"))?;
        let mut f = mgr
            .open(name)
            .ok_or_else(|| anyhow::anyhow!("asset not found in APK: {name}"))?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        Ok(buf)
    }
    #[cfg(not(target_os = "android"))]
    {
        let p = format!("assets/{name}");
        std::fs::read(&p).map_err(|e| anyhow::anyhow!("{p}: {e}"))
    }
}
