from setuptools import setup, find_packages
setup(
    name = "ArmPackManager",
    version = "0.0",
    packages = find_packages(),
    install_requires = ['pycurl>=7.43.0',
                        'pyxdg>=0.25',
                        'beautifulsoup4>=4.4.1',
                        'fuzzywuzzy>=0.10.0'],
    entry_points = {
        'console_scripts' : [
            'pack-manager = ArmPackManager.pack_manager:main'
        ]
    }
)
