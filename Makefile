.PHONY: all run

module_name=libredical.so

# override
OSNICK?=ubuntu22.04
ARCH?=x86_64
OS?=Linux
REDISVERSION?=7.2.4
VERSION?=99.99.99
REDIS_SERVER_PATH?=redis/redis-server

TARGETBASEDIR=target
ifdef RELEASE
RELEASEFLAGS=--release
TARGETDIR=${TARGETBASEDIR}/release
else
TARGETDIR=${TARGETBASEDIR}/debug
endif
module_dest=${TARGETDIR}/${module_name}

all::
	cargo build ${RELEASEFLAGS}

run: all
	redis-server --loadmodule ${module_dest}

test: all
	cargo test --all

clean:
	rm -rf ${module_dest} dump.rdb

distclean:
	rm -rf ${TARGETBASEDIR} dump.rdb redis *.zip

deps:
	pip3 install ramp-packer
	curl -s https://redismodules.s3.amazonaws.com/redis-stack/dependencies/redis-${REDISVERSION}-${OS}-${OSNICK}-${ARCH}.tgz --output redis.tgz
	tar -xpf redis.tgz
	rm *.tgz
	mv redis* redis
	chmod a+x redis/*

pack:
	ramp pack -m ramp.yml target/release/libredical.so -o libredical.Linux-${OSNICK}-${ARCH}.${VERSION}.zip
