FROM rockylinux:8
# Disable SELinux as required by OpenPBS
RUN setenforce 0 2>/dev/null || true

RUN set -ex \
    && yum makecache \
    && yum -y update \
    && yum -y install dnf-plugins-core \
    && yum config-manager --set-enabled powertools \
    && yum -y install \
       gcc \
       make \
       rpm-build \
       libtool \
       hwloc-devel \
       libX11-devel \
       libXt-devel \
       libedit-devel \
       libical-devel \
       ncurses-devel \
       perl \
       postgresql-devel \
       postgresql-contrib \
       python3-devel \
       tcl-devel \
       tk-devel \
       swig \
       expat-devel \
       openssl-devel \
       libXext \
       libXft \
       autoconf \
       automake \
       gcc-c++ \
       git \
       wget \
    && yum -y install \
       expat \
       libedit \
       postgresql-server \
       postgresql-contrib \
       python3 \
       sendmail \
       sudo \
       tcl \
       tk \
       libical \
       procps-ng \
       openssh-server \
       openssh-clients \
    && yum clean all \
    && rm -rf /var/cache/yum


RUN pip3 install Cython pytest
ENV GOSU_VERSION 1.19

RUN set -eux; \
    \
    rpmArch="$(rpm --query --queryformat='%{ARCH}' rpm)"; \
    case "$rpmArch" in \
        aarch64) dpkgArch='arm64' ;; \
        armv[67]*) dpkgArch='armhf' ;; \
        i[3456]86) dpkgArch='i386' ;; \
        ppc64le) dpkgArch='ppc64el' ;; \
        riscv64 | s390x | loongarch64) dpkgArch="$rpmArch" ;; \
        x86_64) dpkgArch='amd64' ;; \
        *) echo >&2 "error: unknown/unsupported architecture '$rpmArch'"; exit 1 ;; \
    esac; \
    \
    wget -O /usr/local/bin/gosu "https://github.com/tianon/gosu/releases/download/$GOSU_VERSION/gosu-$dpkgArch"; \
    wget -O /usr/local/bin/gosu.asc "https://github.com/tianon/gosu/releases/download/$GOSU_VERSION/gosu-$dpkgArch.asc"; \
    \
    export GNUPGHOME="$(mktemp -d)"; \
    gpg --batch --keyserver hkps://keys.openpgp.org --recv-keys B42F6819007F00F88E364FD4036A9C25BF357DD4; \
    gpg --batch --verify /usr/local/bin/gosu.asc /usr/local/bin/gosu; \
    gpgconf --kill all; \
    rm -rf "$GNUPGHOME" /usr/local/bin/gosu.asc; \
    \
    chmod +x /usr/local/bin/gosu; \
    gosu --version; \
    gosu nobody true

ARG PBS_TAG=v22.05.1

RUN set -ex \
    && wget -O openpbs.tar.gz "https://github.com/openpbs/openpbs/archive/refs/tags/${PBS_TAG}.tar.gz" \
    && tar -xzf openpbs.tar.gz \
    && cd openpbs-* \
    && ./autogen.sh \
    && ./configure --prefix=/opt/pbs \
    && make \
    && make install \
    && echo "export PATH=/opt/pbs/bin:/opt/pbs/sbin:\$PATH" | sudo tee /etc/profile.d/pbs.sh \
    && source /etc/profile.d/pbs.sh \
    && /opt/pbs/libexec/pbs_postinstall -i \
    && cd .. \
    && rm -rf openpbs-* openpbs.tar.gz \
    && mkdir -p /var/spool/pbs \
        /var/log/pbs \
        /var/lib/pbs \
        /data \
    && chmod 4755 /opt/pbs/sbin/pbs_iff /opt/pbs/sbin/pbs_rcp

RUN set -eux \
    && wget https://download.open-mpi.org/release/open-mpi/v4.1/openmpi-4.1.5.tar.gz \
    && tar -xvf openmpi-4.1.5.tar.gz \
    && cd openmpi-4.1.5 \
    && ./configure --with-tm=/opt/pbs \
    && make -j 4 \
    && sudo make install \
    && cd .. \
    && rm -rf openmpi-* openmpi-4.1.5.tar.gz

ENV LD_LIBRARY_PATH /usr/lib64/openmpi/lib:/usr/local/lib/$LD_LIBRARY_PATH
ENV PATH /usr/lib64/openmpi/bin:$PATH

COPY pbs.conf /etc/pbs.conf
COPY docker-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/docker-entrypoint.sh
RUN mkdir /mpi_worker
COPY main.py /mpi_worker/main.py


RUN groupadd -g 1001 pbsuser && useradd -u 1001 -g 1001 -m pbsuser
RUN chmod 777 /data && chown -R pbsuser:pbsuser /data

USER pbsuser
RUN curl -LsSf https://astral.sh/uv/install.sh | sh
USER root

ENV PATH /opt/pbs/bin:/opt/pbs/sbin:$PATH

ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]

