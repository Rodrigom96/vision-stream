FROM nvidia/cuda:12.2.0-devel-ubuntu22.04

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

# install pytorch
RUN pip install pip -U
RUN --mount=type=cache,target=/root/.cache/pip \
    pip install  torch==2.2.0 --index-url https://download.pytorch.org/whl/cu121

# install vision stream
WORKDIR /app/vision-stream

COPY src src
COPY pyproject.toml .
COPY setup.py .


RUN --mount=type=cache,target=/root/.cache/pip \
    WITH_CUDA=true pip install .[torch]

# copy example
WORKDIR /app

COPY examples/cuda_rtsp/requirement.txt .
COPY examples/cuda_rtsp/main.py .

RUN --mount=type=cache,target=/root/.cache/pip \
    pip install -r requirement.txt
