fn main() {
  println!("cargo:rustc-link-arg-bin=display=/subsystem:efi_boot_service_driver");
}
