from setuptools import find_packages, setup
from setuptools_rust import Binding, RustExtension

setup(
    name="vision-stream",
    version="0.1.0",
    packages=find_packages(where="python"),
    package_dir={"": "python"},
    rust_extensions=[
        RustExtension(
            "vision_stream._lib",
            path="Cargo.toml",
            binding=Binding.PyO3,
        )
    ],
)
