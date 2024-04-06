import os
from setuptools import find_packages, setup
from setuptools_rust import Binding, RustExtension

deepstream_extra = os.getenv("WITH_DS") is not None

rust_lib_features = []
if deepstream_extra:
    rust_lib_features += ["deepstream"]

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
            features=rust_lib_features
        )
    ],
)
