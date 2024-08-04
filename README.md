# halo2-examples
Simple examples to illustrate the usage of halo2 rust library


## Run examples
cargo test -- --nocapture test_example1 \\
cargo test -- --nocapture test_example2 \\
cargo test -- --nocapture test_example3 

## Print circuit
cargo test --all-features -- --nocapture plot_fibo1
cargo test --all-features -- --nocapture plot_fibo2
