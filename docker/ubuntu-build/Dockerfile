#
# Dockerfile
#
# Used to create an image containing ubuntu linux
# with all required dependencies to build the project.
#

FROM ubuntu:rolling
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get --assume-yes install \
    cargo \
    curl \
    make \
    rustc \
&& useradd \
    --shell /usr/bin/bash \
    --no-create-home \
    ubuntu
USER ubuntu
CMD [ "/usr/bin/bash" ]

