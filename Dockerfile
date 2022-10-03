FROM rust:1-slim

ENV DEBIAN_FRONTEND noninteractive
ENV DEBCONF_NONINTERACTIVE_SEEN true

RUN apt update
RUN apt install -y ffmpeg youtube-dl cmake

COPY . /app
WORKDIR /app/

RUN cargo build --release

CMD ["./target/release/mimicri"]