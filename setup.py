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

from os import getenv
import subprocess
from setuptools import setup
from distutils.version import StrictVersion
from os.path import join, dirname


def build_native(spec):
    build = spec.add_external_build(
        cmd=['cargo', 'build', '--release', '--lib', '--features=cffi'],
        path=join(dirname(__file__), 'rust')
    )

    spec.add_cffi_module(
        module_path='cmsis_pack_manager._native',
        dylib=lambda: build.find_dylib('cmsis_cffi',
                                       in_path='target/release/deps'),
        header_filename=lambda: build.find_header('cmsis.h', in_path='cmsis-cffi')
    )

def run(cmd):
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE)
    stdout, stderr = proc.communicate()
    if proc.returncode != 0:
        raise subprocess.CalledProcessError(
            proc.returncode,
            cmd,
        )
    else:
        return stdout.strip()


with open("requirements.txt") as inreq:
    install_requires = list(inreq)
with open("setup_requirements.txt") as setreq:
    setup_requires = list(setreq)
with open("test_requirements.txt") as testreq:
    test_require = list(testreq)

setup(
    name="cmsis-pack-manager",
    use_scm_version={
        'local_scheme': 'dirty-tag',
        'write_to': 'cmsis_pack_manager/_version.py',
    },
    packages=["cmsis_pack_manager"],
    zip_safe=False,
    platforms='any',
    setup_requires=setup_requires,
    install_requires=install_requires,
    tests_require=test_require,
    entry_points={
        'console_scripts': [
            'pack-manager=cmsis_pack_manager.pack_manager:main'
        ]
    },
    milksnake_tasks=[build_native],
    test_suite="tests"
)
