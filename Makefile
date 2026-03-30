CARGO := cmd.exe /c cargo
DIR   := C:\Users\shive\Desktop\EchoAtlasDeep

.PHONY: run build release clean check fmt

run:
	$(CARGO) run

build:
	$(CARGO) build

release:
	$(CARGO) build --release

check:
	$(CARGO) check

fmt:
	$(CARGO) fmt

clean:
	$(CARGO) clean

# Build release binary then print its location
install: release
	@echo.
	@echo Build complete. Binary at:
	@cmd.exe /c "echo $(DIR)\target\release\rmtide.exe"
