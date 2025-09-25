fn main() {
    println!("cargo:rustc-link-search=native=vendor/raylib");
    println!("cargo:rustc-link-lib=static=raylib");
}

