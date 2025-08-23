#!/bin/bash

# Test script to demonstrate large file skipping functionality

echo "Creating test directory..."
mkdir -p test_large_files
cd test_large_files

echo "Creating test files..."
# Create small files
echo "This is a small file" > small1.txt
echo "Another small file" > small2.txt

# Create a 100KB file (under default 500KB limit)
dd if=/dev/zero of=medium.bin bs=1024 count=100 2>/dev/null
echo "Created 100KB file: medium.bin"

# Create a 600KB file (over default 500KB limit)
dd if=/dev/zero of=large.bin bs=1024 count=600 2>/dev/null
echo "Created 600KB file: large.bin"

# Create a 1MB file
dd if=/dev/zero of=huge.bin bs=1024 count=1024 2>/dev/null
echo "Created 1MB file: huge.bin"

echo ""
echo "File sizes:"
ls -lh *.txt *.bin

echo ""
echo "Test cases:"
echo "1. Default behavior (skip files > 500KB):"
echo "   rcat ."
echo "   Expected: Process small1.txt, small2.txt, medium.bin; Skip large.bin, huge.bin"

echo ""
echo "2. Custom limit of 1MB:"
echo "   rcat --max-file-size 1MB ."
echo "   Expected: Process all except huge.bin"

echo ""
echo "3. Custom limit of 50KB:"
echo "   rcat --max-file-size 50KB ."
echo "   Expected: Process only small1.txt and small2.txt"

echo ""
echo "After building with 'cargo build --release', you can test these scenarios."