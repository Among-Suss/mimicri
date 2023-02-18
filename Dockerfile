FROM rust:1-slim

ENV DEBIAN_FRONTEND noninteractive
ENV DEBCONF_NONINTERACTIVE_SEEN true

RUN apt update
RUN apt install -y ffmpeg cmake python3-pip
RUN pip install youtube-dl

COPY . /app
WORKDIR /app/
# ENV PATH="/app/yt-dlp-linux/:$PATH"

RUN cargo build --release

CMD ["./target/release/mimicri"]
