from setuptools import setup, find_packages

setup(
    name="schemashrink",
    version="0.2.0",
    description="Zero-latency lossless JSON Schema & Tool Definition compressor for LLM agent runtimes",
    author="NousResearch & @byzorky1-sudo",
    packages=find_packages(),
    include_package_data=True,
    python_requires=">=3.8",
    install_requires=[],
    entry_points={
        "hermes_agent.plugins": [
            "schemashrink = schemashrink:register",
        ],
    },
)
