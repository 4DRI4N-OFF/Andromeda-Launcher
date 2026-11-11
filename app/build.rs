fn main() {
    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("ui/branding/andromeda.ico")
        .compile()
        .expect("compile Windows icon resource");
    println!("cargo:rerun-if-changed=ui/branding/andromeda.ico");
    slint_build::compile("ui/main.slint").expect("slint compile");
}
