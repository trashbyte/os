cargo run --features "ci" -- -display none -device "isa-debug-exit,iobase=0xf4,iosize=0x04"
if [ $? -eq 69 ]
then
	exit 0
else
	exit 1
fi
