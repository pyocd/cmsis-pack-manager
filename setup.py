# CMSIS Pack Manager
# Copyright (c) 2017-2020 Arm Limited
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

import os
import sys
from setuptools import setup
from os.path import (join, dirname, realpath)

# Get the directory containing this setup.py. Even though full paths are used below, we must
# chdir in order for setuptools-scm to successfully pick up the version.
SCRIPT_DIR = dirname(realpath(__file__))
os.chdir(SCRIPT_DIR)

# Read the readme file using UTF-8 encoding.
open_args = { 'mode': 'r' }
if sys.version_info[0] > 2:
    # Python 3.x version requires explicitly setting the encoding.
    # Python 2.x version of open() doesn't support the encoding parameter.
    open_args['encoding'] = 'utf-8'

readme_path = os.path.join(SCRIPT_DIR, "README.md")
with open(readme_path, **open_args) as f:
    readme = f.read()

def build_native(spec):
    arch_flags = os.getenv("ARCHFLAGS")
    if arch_flags is not None and ("x86_64" in arch_flags and "arm64" in arch_flags):
        spec.add_external_build(
            cmd=['cargo', 'build', '--release', '--lib', '--target=aarch64-apple-darwin'],
            path=join(dirname(__file__), 'rust')
        )
        spec.add_external_build(
            cmd=['cargo', 'build', '--release', '--lib', '--target=x86_64-apple-darwin'],
            path=join(dirname(__file__), 'rust')
        )
        build = spec.add_external_build(
            cmd=['lipo', '-create', '-output', 'target/release/libcmsis_cffi.dylib',
                 'target/x86_64-apple-darwin/release/libcmsis_cffi.dylib',
                 'target/aarch64-apple-darwin/release/libcmsis_cffi.dylib'],
            path=join(dirname(__file__), 'rust')
        )

        spec.add_cffi_module(
            module_path='cmsis_pack_manager._native',
            dylib=lambda: build.find_dylib('cmsis_cffi',
                                           in_path='target/release'),
            header_filename=lambda: build.find_header('cmsis.h', in_path='cmsis-cffi')
        )
    else:
        build = spec.add_external_build(
            cmd=['cargo', 'build', '--release', '--lib'],
            path=join(dirname(__file__), 'rust')
        )

        spec.add_cffi_module(
            module_path='cmsis_pack_manager._native',
            dylib=lambda: build.find_dylib('cmsis_cffi',
                                           in_path='target/release/deps'),
            header_filename=lambda: build.find_header('cmsis.h', in_path='cmsis-cffi')
        )

setup(
    name="cmsis-pack-manager",
    description="Python manager for CMSIS-Pack index and cache with fast Rust backend",
    long_description=readme,
    long_description_content_type='text/markdown',
    url="https://github.com/pyocd/cmsis-pack-manager",
    license="Apache 2.0",
    use_scm_version={
        'local_scheme': 'dirty-tag',
        'write_to': 'cmsis_pack_manager/_version.py',
    },
    packages=["cmsis_pack_manager"],
    zip_safe=False,
    platforms='any',
    setup_requires=[
        "milksnake>=0.1.2",
        "pytest-runner",
        "setuptools>=40.0",
        "setuptools_scm!=1.5.3,!=1.5.4",
        "setuptools_scm_git_archive",
    ],
    install_requires=[
        "appdirs>=1.4",
        "milksnake>=0.1.2",
        "pyyaml>=5.1",
    ],
    tests_require=[
        "hypothesis",
        "jinja2",
        "mock",
        "pytest",
    ],
    entry_points={
        'console_scripts': [
            'pack-manager=cmsis_pack_manager.pack_manager:main'
        ]
    },
    milksnake_tasks=[build_native],
    milksnake_universal=False,
    test_suite="tests"
)
