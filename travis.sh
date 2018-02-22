
cd rust
cargo test --release
cd ..

python --version
pip2 --version

pip2 install --user --upgrade pip setuptools wheel

python setup.py build

pip2 install --user pytest hypothesis mock jinja2

python setup.py test
