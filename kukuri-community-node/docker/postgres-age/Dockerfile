FROM postgres:16-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        bison \
        build-essential \
        ca-certificates \
        flex \
        git \
        postgresql-server-dev-16 \
    && update-ca-certificates \
    && git clone --branch release/PG16/1.6.0 --depth 1 https://github.com/apache/age.git /tmp/age \
    && cd /tmp/age \
    && make install \
    && rm -rf /tmp/age \
    && apt-get purge -y --auto-remove build-essential git \
    && rm -rf /var/lib/apt/lists/*
