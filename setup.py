# ARM Pack Manager
# Copyright (c) 2017 ARM Limited
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

import sys
import os

from setuptools import setup, find_packages
from os.path import join, dirname

def build_native(spec):
    build = spec.add_external_build(
        cmd=['cargo', 'build', '--release', '--lib', '--features=cffi'],
        path=join(dirname(__file__), 'rust')
    )

    spec.add_cffi_module(
        module_path='cmsis_pack_manager._native',
        dylib=lambda: build.find_dylib('cmsis_cffi', in_path='target/release'),
        header_filename=lambda: build.find_header('cmsis.h', in_path='target')
    )

setup(
    name = "cmsis-pack-manager",
    version = "0.1.0",
    packages = ["cmsis_pack_manager"],
    zip_safe = False,
    platforms = 'any',
    setup_requires = [
        'milksnake>=0.1.2',
        'pytest-runner'],
    install_requires = [
        'appdirs>=1.4',
        'beautifulsoup4>=4.4.1',
        'fuzzywuzzy>=0.10.0',
        'milksnake>=0.1.2'],
    tests_require = [
        'hypothesis', 
        'jinja2',
        'mock',
        'pytest'],
    entry_points = {
        'console_scripts' : [
            'pack-manager = cmsis_pack_manager.pack_manager:main'
        ]
    },
    milksnake_tasks = [build_native],
    test_suite="tests"
)
