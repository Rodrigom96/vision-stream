FROM nvidia/cuda:12.2.0-devel-ubuntu22.04 as build

ENV NVIDIA_DRIVER_CAPABILITIES video,compute,utility

RUN apt-get update &&\
    apt-get install -y\
    build-essential \
    curl \
    python3-pip \
    libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
    gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-qt5

# install rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# upgrade pip
RUN pip install pip -U

# install vision stream
WORKDIR /app/vision-stream

COPY src src
COPY pyproject.toml .
COPY setup.py .

# build wheel
RUN --mount=type=cache,target=/root/.cache/pip \
    WITH_CUDA=true pip wheel .[torch] -w wheels

FROM nvidia/cuda:12.2.0-runtime-ubuntu22.04

ENV NVIDIA_DRIVER_CAPABILITIES video,compute,utility

RUN apt-get update &&\
    apt-get install -y --no-install-recommends\
    python3-pip \
    gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-qt5 \
    # nvcodec deps
    cuda-nvrtc-dev-12-2

# install vision-stream
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install  torch==2.2.0 --index-url https://download.pytorch.org/whl/cu121

RUN --mount=type=bind,from=build,source=/app/vision-stream/wheels,target=/tmp/vision-stream/wheels \
    --mount=type=cache,target=/root/.cache/pip \
    pip install /tmp/vision-stream/wheels/vision_stream-*.whl

# copy example
WORKDIR /app

COPY examples/cuda_rtsp/requirement.txt .
COPY examples/cuda_rtsp/main.py .

RUN --mount=type=cache,target=/root/.cache/pip \
    pip install -r requirement.txt
