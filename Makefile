APP := meow
TARGET := target
RELEASE := $(TARGET)/release
DIST := release

CARGO ?= cargo
CROSS ?= cross

# Target triples
LINUX_TARGET := x86_64-unknown-linux-gnu
MACOS_X64_TARGET := x86_64-apple-darwin
MACOS_ARM_TARGET := aarch64-apple-darwin
WIN_TARGET := x86_64-pc-windows-msvc

# Default
.PHONY: all
all: linux macos-x64 macos-arm windows

.PHONY: build
build:
	$(CARGO) build --release

.PHONY: run
run:
	$(CARGO) run

.PHONY: clean
clean:
	$(CARGO) clean
	rm -rf $(DIST)

.PHONY: install
install:
	install -Dm755 $(RELEASE)/$(APP) /usr/local/bin/$(APP)

.PHONY: uninstall
uninstall:
	rm -f /usr/local/bin/$(APP)

# Ensure release/ directory exists
$(DIST):
	mkdir -p $(DIST)

.PHONY: linux
linux: $(DIST)
	$(CROSS) build --target $(LINUX_TARGET) --release
	cp $(TARGET)/$(LINUX_TARGET)/release/$(APP) $(DIST)/$(APP)-linux-x86_64

.PHONY: macos-x64
macos-x64: $(DIST)
	$(CARGO) build --target $(MACOS_X64_TARGET) --release
	cp $(TARGET)/$(MACOS_X64_TARGET)/release/$(APP) $(DIST)/$(APP)-macos-x86_64

.PHONY: macos-arm
macos-arm: $(DIST)
	$(CARGO) build --target $(MACOS_ARM_TARGET) --release
	cp $(TARGET)/$(MACOS_ARM_TARGET)/release/$(APP) $(DIST)/$(APP)-macos-aarch64

.PHONY: windows
windows: $(DIST)
	$(CROSS) build --target $(WIN_TARGET) --release
	cp $(TARGET)/$(WIN_TARGET)/release/$(APP).exe $(DIST)/$(APP)-windows-x86_64.exe
