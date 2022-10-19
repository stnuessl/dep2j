#
# The MIT License (MIT)
#
# Copyright (c) 2022  Steffen Nuessle
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in
# all copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
# THE SOFTWARE.
#

SHELL := bash -o pipefail

DESTDIR := /usr/local/bin
BUILD_DIR := build
cargo_dir := $(BUILD_DIR)/cargo

bin := dep2j

version_core := \
	$(shell sed -ne 's/version\s\+=\s\+"\([0-9\.]\+\)".*/\1/p' Cargo.toml)

debug_target := $(cargo_dir)/debug/$(bin)
release_target := $(cargo_dir)/release/$(bin)

envfile := $(BUILD_DIR)/env.txt
os_release := $(BUILD_DIR)/os-release.txt
tarball := $(BUILD_DIR)/$(bin)-$(version_core).tar.gz


unix_time := $(shell date --utc +"%s")

#
# Variables for unit tests
#
ut_dir := $(BUILD_DIR)/unit-tests
ut_result := $(ut_dir)/results.txt

#
# Variables for clippy
#
clippy_dir := $(BUILD_DIR)/clippy
clippy_result := $(clippy_dir)/result.txt


#
# Variables for shellcheck
#
shell_scripts := $(shell find . -name "*.sh")

shellcheck_flags := \
	--color=auto \
	--external-sources \
	--format gcc \
	--enable all \
	--norc \
	--shell $(firstword $(notdir $(SHELL)))

shellcheck_dir := $(BUILD_DIR)/shellcheck
shellcheck_output := $(shellcheck_dir)/shellcheck.txt

version_file := $(BUILD_DIR)/versions.txt
version_list = \
	"$(shell cargo --version)" \
	"$(shell rustc --version)" \
	"$(shell make --version)" \
	"$(shell shellcheck --version)" \
	"$(shell curl --version)"

dirs := \
	$(BUILD_DIR) \
	$(shellcheck_dir) \
	$(ut_dir) \
	$(clippy_dir)


#
# Variables for the Artifactory upload
#
ifdef ARTIFACTORY_API_KEY

os_name := $(shell sed -E -n "s/^ID=([a-z0-9\._-]+)\s*$$/\1/p" /etc/os-release)
date	:= $(shell date --utc --date="@$(unix_time)" +"%Y-%m-%d")
time	:= $(shell date --utc --date="@$(unix_time)" +"%H:%M:%S")

artifactory_upload_url := \
	https://nuessle.jfrog.io/artifactory$\
	/dep2j-local$\
	;action=$(GITHUB_RUN_ID)$\
	;branch=$(notdir $(GITHUB_REF))$\
	;uuid=$(shell uuidgen --random)$\
	;commit=$(GITHUB_SHA)$\
	;date=$(date)$\
	;time=$(time)$\
	;timezone=utc$\
	;job=$(GITHUB_JOB)$\
	;os=$(os_name)$\
	;version=$(version_core)$\
	/$(os_name)$\
	/$(date)$\
	/$(time)

endif

ifneq ($(MAKEFILE_COLOR), 0)

red			:= \e[1;31m
green		:= \e[1;32m
yellow		:= \e[1;33m
blue		:= \e[1;34m
magenta		:= \e[1;35m
cyan		:= \e[1;36m
reset		:= \e[0m

endif

all: debug

debug: $(debug_target)

release: $(release_target)

$(debug_target):
	@printf "$(blue)Building [ $@ ]$(reset)\n"
	cargo build

$(release_target):
	@printf "$(blue)Building [ $@ ]$(reset)\n"
	cargo build --release

unit-tests: $(ut_result)

$(ut_result): | $(dirs)
	@printf "$(blue)Generating [ $@ ]$(reset)\n"
	cargo test --release 2>&1 | tee $@ || (rm -f $@ && false)

clippy: $(clippy_result)

$(clippy_result): | $(dirs)
	@printf "$(blue)Generating [ $@ ]$(reset)\n"
	cargo clippy --release -- --deny=clippy::all 2>&1 | tee $@ \
		|| (rm -f $@ && false)

install: $(release_target)
	cp -f $< $(DESTDIR)

uninstall:
	rm -f $(DESTDIR)/$(bin)

cargo-clean:
	cargo clean

clean:
	rm -rf build/

$(envfile): | $(dirs)
	@env \
		$(if $(ARTIFACTORY_API_KEY),ARTIFACTORY_API_KEY=) \
		$(if $(DOCKER_USERNAME),DOCKER_USERNAME=) \
		$(if $(DOCKER_PASSWORD),DOCKER_PASSWORD=) \
		> $@

$(os_release): /etc/os-release | $(dirs)
	@cp -f $< $@

shellcheck: $(shellcheck_output)

$(shellcheck_output): $(shell_scripts) | $(dirs)
	@printf "$(yellow)Generating [ $@ ]$(reset)\n"
	$(Q)shellcheck $(shellcheck_flags) $^ | tee $@ || (rm -f $@ && false)

$(version_file): | $(dirs)
	@printf "%s\n--\n" $(version_list) > $@ || (rm -f $@ && false)

$(dirs):
	mkdir -p $@

$(tarball): \
		$(release_target) \
		$(debug_target) \
		$(envfile) \
		$(os_release) \
		$(shellcheck_output) \
		$(ut_result) \
		$(clippy_result) \
		$(version_file)
	@printf "$(magenta)Packaging [ $@ ]$(reset)\n"
	$(Q)find -H $^ -type f -size +0 \
		| sed -e 's/^\(\.\/\)\?$(@D)\///g' \
		| tar --create --file $@ --gzip --directory $(@D) --files-from -

artifactory-upload: $(tarball)
	@printf "$(magenta)Uploading [ $^ ]$(reset)\n"
ifdef ARTIFACTORY_API_KEY
	$(Q)curl \
		--silent \
		--show-error \
		--write-out "\n" \
		--request PUT \
		--header "X-JFrog-Art-Api:${ARTIFACTORY_API_KEY}" \
		--header "X-Checksum-Deploy:false" \
		--header "X-Checksum-Sha256:$$(sha256sum $^ | cut --fields=1 -d " ")" \
		--header "X-Checksum-Sha1:$$(sha1sum $^ | cut --fields=1 -d " ")" \
		--upload-file $^ \
		"$(artifactory_upload_url)/$(^F)"
	@printf "$(green)Uploaded [ $^ ]$(reset)\n"
else
	@printf "** ERROR: $@: \"ARTIFACTORY_API_KEY\" not specified\n"
	@false
endif


.PHONY: \
	$(release_target) \
	$(debug_target) \
	$(ut_result) \
	all \
	artifactory-upload \
	cargo-clean \
	clean \
	debug \
	lint \
	release \
	shellcheck \
	unit-tests

.SILENT: \
	$(dirs)

