# Compiler settings
CC = gcc
CFLAGS = -Wall -Wextra -I include # -Werror

# Build directory
BUILD = build

# Automatically collect all .c files in src/
SRC = $(wildcard src/*.c)

# Generate object file list inside build/
OBJ = $(patsubst src/%.c, $(BUILD)/%.o, $(SRC))

# Output executable name
TARGET = deflate

# Build final executable
$(TARGET): $(OBJ)
	$(CC) $(CFLAGS) -o $(BUILD)/$(TARGET) $(OBJ) -lz

# Rule to build each .o file
$(BUILD)/%.o: src/%.c | $(BUILD)
	$(CC) $(CFLAGS) -c $< -o $@

# Ensure the build directory exists
$(BUILD):
	mkdir -p $(BUILD)

# Cleanup
clean:
	rm -rf $(BUILD)
