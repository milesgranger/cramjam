FROM quay.io/pypa/manylinux1_x86_64

ENV HOME /root
ENV PATH $HOME/.cargo/bin:$PATH

# Otherwise `cargo new` errors
ENV USER root

RUN curl https://sh.rustup.rs -sSf | \
    sh -s -- --default-toolchain stable -y

# Add all supported python versions
ENV PATH /opt/python/cp35-cp35m/bin/:/opt/python/cp36-cp36m/bin/:/opt/python/cp37-cp37m/bin/:/opt/python/cp38-cp38/bin/:$PATH

RUN python3 -m pip install --no-cache-dir cffi
RUN python3 -m pip install maturin

WORKDIR /io
