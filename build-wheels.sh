#!/bin/bash
set -e -x

function install_rust {
    curl https://static.rust-lang.org/rustup.sh > /tmp/rustup.sh
    chmod +x /tmp/rustup.sh
    /tmp/rustup.sh -y --disable-sudo --channel=$1
}

function clean_project {
    # Remove compiled files that might cause conflicts
    pushd /io/
    rm -rf .cache .eggs rust_fst/_ffi.py build *.egg-info
    find ./ -name "__pycache__" -type d -print0 |xargs -0 rm -rf
    find ./ -name "*.pyc" -type f -print0 |xargs -0 rm -rf
    find ./ -name "*.so" -type f -print0 |xargs -0 rm -rf
    popd
}

RUST_CHANNEL=nightly

if [[ $1 == "osx" ]]; then
    pip2 install --user -U pip setuptools wheel milksnake
    install_rust $RUST_CHANNEL
    python2 setup.py bdist_wheel
    pip2 install --user -U cffi pytest mock hypothesis jinja2
    pip2 install --user -v cmsis_pack_manager --no-index -f ./dist
    pushd tests
    python2 -m pytest
    popd
else
    PYBIN=/opt/python/cp27-cp27m/bin
    # Clean build files
    clean_project

    install_rust $RUST_CHANNEL

    # Remove old wheels
    rm -rf /io/dist/* || echo "No old wheels to delete"

    # Install libraries needed for compiling the extension
    yum -q -y install libffi-devel

    # Compile wheel
    ${PYBIN}/python -m pip wheel /io/ -w /dist/

    # Move pure wheels to target directory
    mkdir -p /io/dist
    mv /dist/*any.whl /io/dist || echo "No pure wheels to move"

    # Bundle external shared libraries into the wheel
    for whl in /dist/*.whl; do
        auditwheel repair $whl -w /io/dist/
    done

    # Set permissions on wheels
    chmod -R a+rw /io/dist

    # Install packages and test with all Python versions
    ${PYBIN}/python -m pip install cffi pytest mock hypothesis jinja2
    ${PYBIN}/python -m pip install cmsis_pack_manager --no-index -f /io/dist
    ${PYBIN}/python -m pytest /io/tests
fi
