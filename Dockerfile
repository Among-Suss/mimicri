FROM rust:1-slim

ENV DEBIAN_FRONTEND noninteractive
ENV DEBCONF_NONINTERACTIVE_SEEN true

RUN apt update
RUN apt install -y ffmpeg cmake python3-pip wget

RUN wget https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux
RUN mv yt-dlp_linux /usr/local/bin/yt-dlp
RUN chmod +x /usr/local/bin/yt-dlp

COPY . /app
WORKDIR /app/

RUN cargo build --release

CMD ["./target/release/mimicri"]
