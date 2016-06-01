from setuptools import setup, find_packages
setup(
    name = "ArmPackManager",
    version = "0.0",
    packages = find_packages(),
    install_requires = ['pycurl>=7.43.0',
                        'pyxdg>=0.25']
)
