extern crate embed_resource;

fn main() {
    #[cfg(target_os = "windows")]
    embed_resource::compile("assets/logo/logo.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();

    #[cfg(target_os = "windows")]
    embed_resource::compile("assets/logo/app.manifest.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
