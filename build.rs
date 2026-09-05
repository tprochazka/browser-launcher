fn main() {
    println!("cargo:rerun-if-changed=assets/browser-launcher-v2.ico");
    println!("cargo:rerun-if-changed=assets/browser-launcher.rc");
    println!("cargo:rerun-if-changed=assets/browser-launcher.manifest");
    embed_resource::compile("assets/browser-launcher.rc", embed_resource::NONE)
        .manifest_required()
        .expect("Windows resources could not be embedded");
}
