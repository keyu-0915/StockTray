FROM ubuntu:22.04 AS downloader

ARG FUTU_OPEND_VERSION=10.8.6818
ADD --checksum=sha256:9bf1f50cdc9db2d030bd2c1db4daf4e096d6810a1e5bdb063b9395f94051e716 \
    https://softwaredownload.futunn.com/Futu_OpenD_10.8.6818_Ubuntu18.04.tar.gz \
    /tmp/opend.tar.gz

RUN mkdir -p /opt/futu-opend \
    && tar --extract --gzip --file /tmp/opend.tar.gz \
        --directory /opt/futu-opend \
        --strip-components=2 \
        "Futu_OpenD_${FUTU_OPEND_VERSION}_Ubuntu18.04/Futu_OpenD_${FUTU_OPEND_VERSION}_Ubuntu18.04"

FROM ubuntu:22.04

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates netcat-openbsd \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 --shell /usr/sbin/nologin futu \
    && mkdir -p /run/futu-opend /var/lib/futu-opend /home/futu/.com.futunn.FutuOpenD \
    && chown -R futu:futu /run/futu-opend /var/lib/futu-opend /home/futu/.com.futunn.FutuOpenD

COPY --from=downloader --chown=futu:futu /opt/futu-opend/ /opt/futu-opend/
COPY --chown=futu:futu entrypoint.sh /usr/local/bin/futu-opend-entrypoint

RUN chmod 0755 /opt/futu-opend/FutuOpenD /usr/local/bin/futu-opend-entrypoint

USER futu
WORKDIR /opt/futu-opend

ENV FUTU_API_PORT=32179 \
    FUTU_LOG_LEVEL=info \
    FUTU_AUTO_HOLD_QUOTE_RIGHT=0

VOLUME ["/home/futu/.com.futunn.FutuOpenD", "/var/lib/futu-opend"]
EXPOSE 32179

ENTRYPOINT ["/usr/local/bin/futu-opend-entrypoint"]
