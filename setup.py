# CMSIS Pack Manager
# Copyright (c) 2017-2020 Arm Limited
# Copyright (c) 2021 Mathias Brossard
# Copyright (c) 2021 Chris Reed
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
from setuptools import setup
from pathlib import Path

# Get the directory containing this setup.py. Even though full paths are used below, we must
# chdir in order for setuptools-scm to successfully pick up the version.
SCRIPT_DIR = Path(__file__).resolve().parent
os.chdir(SCRIPT_DIR)

RUST_DIR = str(SCRIPT_DIR / "rust")

def build_native(spec):
    arch_flags = os.getenv("ARCHFLAGS")
    if arch_flags is not None and ("x86_64" in arch_flags and "arm64" in arch_flags):
        spec.add_external_build(
            cmd=['cargo', 'build', '--release', '--lib', '--target=aarch64-apple-darwin'],
            path=RUST_DIR
        )
        spec.add_external_build(
            cmd=['cargo', 'build', '--release', '--lib', '--target=x86_64-apple-darwin'],
            path=RUST_DIR
        )
        build = spec.add_external_build(
            cmd=['lipo', '-create', '-output', 'target/release/libcmsis_cffi.dylib',
                 'target/x86_64-apple-darwin/release/libcmsis_cffi.dylib',
                 'target/aarch64-apple-darwin/release/libcmsis_cffi.dylib'],
            path=RUST_DIR
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
            path=RUST_DIR
        )

        spec.add_cffi_module(
            module_path='cmsis_pack_manager._native',
            dylib=lambda: build.find_dylib('cmsis_cffi',
                                           in_path='target/release/deps'),
            header_filename=lambda: build.find_header('cmsis.h', in_path='cmsis-cffi')
        )

setup(
    milksnake_tasks=[build_native],
    milksnake_universal=False,
)
