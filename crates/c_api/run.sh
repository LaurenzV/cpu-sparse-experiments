RUSTFLAGS='-C target-cpu=native' cargo build --release
cbindgen --config cbindgen.toml --output cpu_sparse.h