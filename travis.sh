
cd rust
cargo test --release
cd ..

python setup.py build
python setup.py test
