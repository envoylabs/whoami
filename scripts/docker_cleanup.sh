#!/bin/bash

set -e

# aggressively prune data volumes, artifacts and orphans
docker system prune -a --volumes