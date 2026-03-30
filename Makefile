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
