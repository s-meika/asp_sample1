fn main(){
    println!("cargo:rustc-link-lib=kernel");
    println!("cargo:rustc-link-lib=syssvc");
    println!("cargo:rustc-link-lib=cfg");

    println!("cargo:rustc-link-search=./src/cfg");
}