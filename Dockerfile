FROM rust:1.93

RUN apt-get update && \
  apt-get install -y g++ pkg-config libx11-dev libasound2-dev libudev-dev libxkbcommon-x11-0 libdbus-1-dev
# libwayland-dev libxkbcommon-dev \
# libvulkan1 mesa-vulkan-drivers vulkan-tools

RUN rustup component add clippy

RUN curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to /bin

WORKDIR /app
COPY . .

CMD ["tail", "-f", "/dev/null"]
