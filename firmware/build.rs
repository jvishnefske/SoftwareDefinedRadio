//! Build script for SDR firmware
//!
//! Handles:
//! - Memory layout configuration
//! - Optional USB PD library linking (X-CUBE-TCPP)

fn main() {
    // Tell Cargo to re-run this if the linker script changes
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=build.rs");

    // Link memory.x from project directory
    println!("cargo:rustc-link-search={}", std::env::var("CARGO_MANIFEST_DIR").unwrap());

    // Optional: Link X-CUBE-TCPP for USB Power Delivery
    #[cfg(feature = "usb-pd")]
    {
        println!("cargo:rerun-if-changed=vendor/x-cube-tcpp/lib/libusbpd_core_cm4.a");

        // Add the library search path
        println!(
            "cargo:rustc-link-search={}/vendor/x-cube-tcpp/lib",
            std::env::var("CARGO_MANIFEST_DIR").unwrap()
        );

        // Link the USB PD library
        println!("cargo:rustc-link-lib=static=usbpd_core_cm4");
    }
}
