
# Dockerfile
#
# Used to create an image containing Fedora
# with all required dependencies to build the project.
#

FROM fedora:latest
RUN dnf install --assumeyes \
    cargo \
    clippy \
    curl \
    make \
    rust \
    ShellCheck \
&& dnf clean dbcache --assumeyes \
&& useradd \
    --shell /usr/bin/bash \
    --no-create-home \
    fedora
USER fedora
CMD [ "/usr/bin/bash" ]

