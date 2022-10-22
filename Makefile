# Taken from https://github.com/ianstormtaylor/makefile-help
# Show this help prompt.
help:
	@ echo
	@ echo '  Usage:'
	@ echo ''
	@ echo '    make <target> [flags...]'
	@ echo ''
	@ echo '  Targets:'
	@ echo ''
	@ awk '/^#/{ comment = substr($$0,3) } comment && /^[a-zA-Z][a-zA-Z0-9_-]+ ?:/{ print "   ", $$1, comment }' $(MAKEFILE_LIST) | column -t -s ':' | sort
	@ echo ''
	@ echo '  Flags:'
	@ echo ''
	@ awk '/^#/{ comment = substr($$0,3) } comment && /^[a-zA-Z][a-zA-Z0-9_-]+ ?\?=/{ print "   ", $$1, $$2, comment }' $(MAKEFILE_LIST) | column -t -s '?=' | sort
	@ echo ''


.DEFAULT_GOAL = build

# Build the project for release, default is for debugging
release ?= 0

# Show output when running the tests, default is 0
showoutput ?= 0

# `cargo build` flags
BUILD_FLAGS ?=
# `cargo test` flags
TEST_FLAGS ?=


ifeq ($(release),1)
	BUILD_FLAGS := $(BUILD_FLAGS) -r
endif

ifeq ($(showoutput),1)
	TEST_FLAGS := $(TEST_FLAGS) --nocapture
endif


# Builds the project
.PHONY: build
build:
	cargo build $(BUILD_FLAGS)

# Run the tests
.PHONY: test
test:
	cargo test -- $(TEST_FLAGS)

# Clean all artifacts
.PHONY: clean
clean:
	cargo clean

# Prints Makefile variables - for debugging
.PHONY: debug
debug:
	@echo "BUILD_FLAGS=$(BUILD_FLAGS)"
	@echo "TEST_FLAGS=$(TEST_FLAGS)"
