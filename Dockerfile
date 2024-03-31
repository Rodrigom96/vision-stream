FROM nvcr.io/nvidia/deepstream:6.4-samples-multiarch

RUN apt-get update &&\
    apt-get install -y\
    build-essential \
    curl \
    python3-pip \
    libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
    gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN /opt/nvidia/deepstream/deepstream/user_additional_install.sh \
    && apt remove -y gstreamer1.0-plugins-ugly

WORKDIR /app

RUN pip install pip -U
RUN pip install torch==2.2.0 --index-url https://download.pytorch.org/whl/cu121

COPY deepstream-sys deepstream-sys
COPY python python
COPY src src
COPY Cargo.lock .
COPY Cargo.toml .
COPY pyproject.toml .
COPY setup.py .

ENV LD_LIBRARY_PATH="/opt/nvidia/deepstream/deepstream/lib/:${LD_LIBRARY_PATH}"

#Use the libtorch provided by PyTorch
ENV LIBTORCH_USE_PYTORCH=1 
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install .
