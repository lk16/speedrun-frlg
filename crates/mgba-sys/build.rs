use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=csrc/shim.c");
    println!("cargo:rerun-if-env-changed=MGBA_PREFIX");

    let prefix = PathBuf::from(env::var("MGBA_PREFIX").expect(
        "MGBA_PREFIX is not set. It comes from /etc/profile.d/10-frlg.sh; \
         run bin/frlg-doctor if the sandbox environment looks wrong.",
    ));
    let include = prefix.join("include");
    let lib = prefix.join("lib");

    assert!(
        include.join("mgba/core/core.h").is_file(),
        "no mgba headers under {}",
        include.display()
    );
    assert!(
        lib.join("libmgba.so").exists(),
        "no libmgba.so under {}",
        lib.display()
    );

    cc::Build::new()
        .file("csrc/shim.c")
        .include(&include)
        // gnu11, not c11: strict ANSI hides PATH_MAX in <limits.h>, and
        // mgba's directories.h uses it at file scope.
        .std("gnu11")
        .warnings(true)
        .extra_warnings(true)
        .compile("frlg_shim");

    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=dylib=mgba");
    // LD_LIBRARY_PATH already carries this directory, but an rpath means a
    // binary copied out of the workspace still resolves the library.
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib.display());
}
