VERSION := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*= *"\(.*\)"/\1/')

run:
	cargo r

bundle:
	cargo bundle --release

setup:
	mkdir -p .git/hooks
	cp -rf scripts/pre-commit .git/hooks
	chmod +x .git/hooks/pre-commit

# Build, sign, notarize, and produce GoKey-v<VERSION>.zip ready for release.
# Requires: cargo-bundle, a valid "Developer ID Application" cert, and the
# AC_PASSWORD keychain profile configured via xcrun notarytool.
release: bundle
	bash scripts/release
	cd target/release/bundle/osx && \
	  ditto -c -k --keepParent GoKey.app GoKey-v$(VERSION).zip
	@echo "Release asset: target/release/bundle/osx/GoKey-v$(VERSION).zip"
	@echo "SHA256: $$(shasum -a 256 target/release/bundle/osx/GoKey-v$(VERSION).zip | awk '{print $$1}')"

# Update Casks/goxkey.rb with the SHA256 of the just-built release zip.
# Run after `make release` before tagging.
update-cask:
	$(eval SHA256 := $(shell shasum -a 256 target/release/bundle/osx/GoKey-v$(VERSION).zip | awk '{print $$1}'))
	sed -i '' 's/version ".*"/version "$(VERSION)"/' Casks/goxkey.rb
	sed -i '' 's/sha256 ".*"/sha256 "$(SHA256)"/' Casks/goxkey.rb
	@echo "Casks/goxkey.rb updated → version=$(VERSION) sha256=$(SHA256)"

# ──────────────────────────────────────────────
# IBus engine (Linux)
# ──────────────────────────────────────────────

IBUS_COMPONENT_DIR ?= /usr/share/ibus/component
IBUS_BIN_DIR      ?= $(HOME)/.local/bin

.PHONY: ibus-setup ibus-uninstall ibus-reinstall

ibus-setup: ibus-build ibus-install-xml ibus-restart

ibus-reinstall: ibus-build ibus-install ibus-install-xml ibus-restart

ibus-build:
	cargo build --release -p goxkey-ibus
	install -d $(IBUS_BIN_DIR)
	install -m 755 target/release/goxkey-ibus $(IBUS_BIN_DIR)/goxkey-ibus

ibus-install:
	install -d $(IBUS_BIN_DIR)
	install -m 755 target/release/goxkey-ibus $(IBUS_BIN_DIR)/goxkey-ibus

ibus-install-xml:
	sudo install -d $(IBUS_COMPONENT_DIR)
	{ \
	  echo '<?xml version="1.0" encoding="utf-8"?>'; \
	  echo '<component>'; \
	  echo '  <name>org.freedesktop.IBus.Goxkey</name>'; \
	  echo '  <description>Goxkey Component</description>'; \
	  echo "  <exec>$(IBUS_BIN_DIR)/goxkey-ibus</exec>"; \
	  echo '  <version>$(VERSION)</version>'; \
	  echo '  <author>Huy Tran</author>'; \
	  echo '  <license>MIT</license>'; \
	  echo '  <homepage>https://github.com/huytd/goxkey</homepage>'; \
	  echo '  <textdomain>goxkey</textdomain>'; \
	  echo '  <engines>'; \
	  echo '    <engine>'; \
	  echo '      <name>goxkey</name>'; \
	  echo '      <language>vi</language>'; \
	  echo '      <license>MIT</license>'; \
	  echo '      <author>Huy Tran</author>'; \
	  echo '      <layout>us</layout>'; \
	  echo '      <longname>GoKey Vietnamese</longname>'; \
	  echo '      <description>Vietnamese Input Method (Telex, VNI, Telex+VNI)</description>'; \
	  echo '      <rank>0</rank>'; \
	  echo '    </engine>'; \
	  echo '  </engines>'; \
	  echo '</component>'; \
	} | sudo tee $(IBUS_COMPONENT_DIR)/goxkey.xml > /dev/null
	@echo "goxkey.xml installed to $(IBUS_COMPONENT_DIR)"

ibus-restart:
	@echo "Restarting IBus..."
	ibus restart 2>/dev/null || (ibus-daemon --daemonize 2>/dev/null) || true
	@echo "Done. Select Gõ Key in your input method settings."

ibus-uninstall:
	rm -f $(IBUS_COMPONENT_DIR)/goxkey.xml
	rm -f $(IBUS_BIN_DIR)/goxkey-ibus
	ibus restart 2>/dev/null || true
	@echo "Goxkey IBus engine uninstalled."
