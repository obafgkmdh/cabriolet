#!/usr/bin/gdb -x
import gdb
import struct
import os

register_names = [
    "rax", "rbx", "rcx", "rdx", "rsi", "rdi", "rbp", "rsp",
    "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15", "rip",
    "eflags", "cs", "ss", "ds", "es", "fs", "gs",
    "st0", "st1", "st2", "st3", "st4", "st5", "st6", "st7",
    "fctrl", "fstat", "ftag", "fiseg", "fioff", "foseg", "fooff", "fop", "mxcsr",
]

gdb.execute("file target/debug/cabriolet_examples")
gdb.execute("set mem inaccessible-by-default off")

saved_regions = {}
saved_values = {}
saved_registers = {}


def reset_saved_memory():
    saved_regions.clear()
    saved_values.clear()


def save_value(inferior, val):
    addr = val.address
    size = val.type.sizeof
    b = inferior.read_memory(addr, size).tobytes()
    saved_values[(addr, size)] = b


class Checkpoint(gdb.Breakpoint):
    def __init__(self, name):
        gdb.Breakpoint.__init__(self, "checkpoint")

    def stop(self):
        reset_saved_memory()
        inferior = gdb.selected_inferior()

        # 1: save all registers
        for name in register_names:
            value = int(gdb.parse_and_eval(f"${name}"))
            saved_registers[name] = value

        # 2: save all writeable memory regions (stack, heap, etc.)
        mappings = gdb.execute("info proc mappings", to_string=True)
        for line in mappings.splitlines():
            if not line.strip().startswith("0x"):
                continue
            start, end, size, offset, perms, *objfile = line.split(maxsplit=5)
            start = int(start[2:], 16)
            size = int(size[2:], 16)
            offset = int(offset[2:], 16)
            if perms[1] != "w":
                # not writeable
                continue
            print(hex(start), hex(size), hex(offset), perms, *objfile)
            saved_regions[(start, size)] = inferior.read_memory(start, size).tobytes()

        # 3: find non-volatile and timely values
        frame = gdb.newest_frame()
        while frame is not None:
            print("in frame:", frame.name())
            try:
                block = frame.block()
            except RuntimeError:
                # no block for this frame
                frame = frame.older()
                continue
            while block:
                for symbol in block:
                    if not (symbol.is_argument or symbol.is_variable):
                        continue
                    name = symbol.name
                    value = symbol.value(frame)
                    vtype = value.type
                    if vtype.code != gdb.TYPE_CODE_STRUCT:
                        continue
                    if "secrets_structs::Labeled" not in vtype.name:
                        continue
                    # value is a labeled object, save corresponding memory
                    print(f"saving {name}")
                    val = value["val"]
                    metadata = value["metadata"]
                    is_some = int(val['']) == 1
                    if is_some:
                        save_value(inferior, val[''])
                        some = val["Some"]
                        # if we were to follow pointers it would happen here
                        save_value(inferior, some)
                    else:
                        save_value(inferior, val)
                    save_value(inferior, metadata)
                block = block.superblock
            frame = frame.older()
        # save checkpoint to file
        with open("checkpoint.bin", "wb") as f:
            f.write(struct.pack("<Q", len(saved_registers)))
            for name, value in saved_registers.items():
                f.write(name.encode().ljust(8, b"\x00"))
                f.write(struct.pack("<Q", value))
            f.write(struct.pack("<Q", len(saved_regions)))
            for (start, size), mem in saved_regions.items():
                f.write(struct.pack("<QQ", start, size))
                f.write(mem)
            f.write(struct.pack("<Q", len(saved_values)))
            for (start, size), mem in saved_values.items():
                f.write(struct.pack("<QQ", start, size))
                f.write(mem)
        # automatically continue program execution
        return False

Checkpoint("checkpoint")

if os.path.exists("checkpoint.bin"):
    gdb.execute("start")
    inferior = gdb.selected_inferior()
    # read checkpoint from file
    with open("checkpoint.bin", "rb") as f:
        (n_registers,) = struct.unpack("<Q", f.read(8))
        for i in range(n_registers):
            name = f.read(8).rstrip(b"\x00").decode()
            (value,) = struct.unpack("<Q", f.read(8))
            gdb.execute(f"set ${name}={value}")
        (n_saved_regions,) = struct.unpack("<Q", f.read(8))
        for i in range(n_saved_regions):
            (start, size) = struct.unpack("<QQ", f.read(16))
            mem = f.read(size)
            try:
                inferior.write_memory(start, mem, size)
            except gdb.MemoryError:
                # will probably need to mmap the page
                print("uhhhhhhhhhhhhhhhhhhhh", hex(start))
        (n_saved_values,) = struct.unpack("<Q", f.read(8))
        for i in range(n_saved_values):
            (start, size) = struct.unpack("<QQ", f.read(16))
            mem = f.read(size)
            try:
                inferior.write_memory(start, mem, size)
            except gdb.MemoryError:
                # will probably need to mmap the page
                print("uhhhhhhhhhhhhhhhhhhhhhhhhhhhhh", hex(start))
    gdb.execute("continue")
else:
    gdb.execute("run")
