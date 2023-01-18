run:
	cargo r

bundle:
	cargo bundle

setup:
	mkdir -p .git/hooks
	cp -rf scripts/pre-commit .git/hooks
	chmod +x .git/hooks/pre-commit
