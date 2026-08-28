fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-arg-bins=/DELAYLOAD:vulkan-1.dll");
        println!("cargo:rustc-link-arg=/DELAYLOAD:vulkan-1.dll");
        println!("cargo:rustc-link-lib=delayimp");
    }
    tauri_build::build();
}

