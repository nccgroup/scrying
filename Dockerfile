FROM debian:buster

RUN apt-get update && apt-get upgrade -y && \
	apt-get install -y libwebkit2gtk-4.0-37 xvfb

COPY target/release/scrying /

#ENTRYPOINT ["/usr/bin/xvfb-run", "-a", "/scrying"]
#ENTRYPOINT ["/scrying"]
#ENTRYPOINT /usr/bin/xvfb-run -a -s "-server 0 1280x720x24" /scrying
ENTRYPOINT ["/usr/bin/xvfb-run", "-a", "-s", "-server 0 1280x720x24", "/scrying"]
CMD ["--help"]
