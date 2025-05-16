submit:
	if [ -f submit.zip ]; then rm submit.zip; fi
	cargo build --release
	@TMP_DIR=$$(mktemp -d) && \
	mkdir -p "$$TMP_DIR/rust-git" && \
	cp -r ../rust-git/src "$$TMP_DIR/rust-git/" && \
	mkdir -p "$$TMP_DIR/rust-git/target/release" && \
	cp ../rust-git/target/release/rust-git "$$TMP_DIR/rust-git/target/release/" && \
	cp ../rust-git/Cargo.toml ../rust-git/Cargo.lock "$$TMP_DIR/rust-git/" && \
	(cd "$$TMP_DIR" && zip -qr submit.zip rust-git) && \
	mv "$$TMP_DIR/submit.zip" . && \
	rm -rf "$$TMP_DIR" && \
	echo "submit.zip created successfully!"

.PHONY: submit