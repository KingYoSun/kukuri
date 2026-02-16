FROM postgres:16-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        bison \
        build-essential \
        ca-certificates \
        flex \
        git \
        postgresql-server-dev-16 \
        wget \
    && update-ca-certificates \
    && wget -O /tmp/groonga-apt-source-latest-bookworm.deb https://packages.groonga.org/debian/groonga-apt-source-latest-bookworm.deb \
    && apt-get install -y --no-install-recommends /tmp/groonga-apt-source-latest-bookworm.deb \
    && rm -f /tmp/groonga-apt-source-latest-bookworm.deb \
    && apt-get update \
    && apt-get install -y --no-install-recommends postgresql-16-pgdg-pgroonga \
    && git clone --branch release/PG16/1.6.0 --depth 1 https://github.com/apache/age.git /tmp/age \
    && cd /tmp/age \
    && make install \
    && rm -rf /tmp/age \
    && apt-get purge -y --auto-remove build-essential git wget \
    && rm -rf /var/lib/apt/lists/*
