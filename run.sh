#!/bin/bash

#podman run -d -p 8123:8123 python-test
podman run -it --rm -p 8080:8080 --name dcss --replace dcss-webtiles
