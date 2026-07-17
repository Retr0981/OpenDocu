from setuptools import setup, find_packages

setup(
    name="opendocu",
    version="0.1.0",
    description="Python bindings for OpenDocu — high-performance document reduction in Rust.",
    long_description=(
        "OpenDocu is a document reduction platform built in Rust. "
        "This package loads the native libopendocu library via ctypes."
    ),
    long_description_content_type="text/plain",
    license="MIT",
    url="https://opendocu.org",
    packages=find_packages(),
    python_requires=">=3.8",
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
        "Topic :: Text Processing",
    ],
)
