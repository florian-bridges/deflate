
import zlib


IN_FILE_PATH = "data/hello_world.txt"
OUT_FILE_PATH = "test_out/hello_world.txt.gz"


# gzip file header variables
GZIP_HEADER_ID1 = 0x1f
GZIP_HEADER_ID2 = 0x8b

# compression_method 
GZIP_HEADER_CM_DEFLATE = 0x08
GZIP_HEADER_FLG_FTEXT = 0x00
GZIP_HEADER_FLG_FHCRC = 0x00
GZIP_HEADER_FLG_FEXTRA = 0x00
GZIP_HEADER_FLG_FNAME = 0x00
GZIP_HEADER_FLG_FCOMMENT = 0x00

GZIP_HEADER_MTIME = 0x00
GZIP_HEADER_XFL = 0x00
GZIP_HEADER_OS = 0x03

def to_gzip_numerical(value, num_bytes):

    if value > 2**(num_bytes * 8):
        raise ValueError(f"length of bitstream has to be less than {2**(num_bytes * 8)} bit is {value}")

    out_bytes = bytearray()

    for i in range(num_bytes):
        out_bytes.append(
            (value >> (8 * i)) & 0xFF
        )

    return out_bytes


def build_header(gzip_compression_method=GZIP_HEADER_CM_DEFLATE):
    
    out_stream = bytearray()

    out_stream.append(GZIP_HEADER_ID1)
    out_stream.append(GZIP_HEADER_ID2)
    out_stream.append(GZIP_HEADER_CM_DEFLATE)

    gzip_header_flag = 0x00
    gzip_header_flag |= GZIP_HEADER_FLG_FTEXT
    gzip_header_flag |= GZIP_HEADER_FLG_FHCRC << 1
    gzip_header_flag |= GZIP_HEADER_FLG_FEXTRA << 2
    gzip_header_flag |= GZIP_HEADER_FLG_FNAME << 3
    gzip_header_flag |= GZIP_HEADER_FLG_FCOMMENT << 4
    out_stream.append(gzip_header_flag)

    out_stream += to_gzip_numerical(GZIP_HEADER_MTIME, 4)

    out_stream.append(GZIP_HEADER_XFL)
    out_stream.append(GZIP_HEADER_OS)

    return out_stream

def build_footer(in_stream):

    out_stream = bytearray()

    crc = zlib.crc32(in_stream)
    out_stream += to_gzip_numerical(crc, 4)


    len_bitstream = len(in_stream)
    out_stream += to_gzip_numerical(len_bitstream, 4)

    return out_stream



def block_type_0(in_stream, is_last=1):

    block_type = 0
    out_stream = bytearray()

    block_header = 0x00
    block_header |= is_last << 0
    block_header |= block_type  << 1

    out_stream.append(block_header)

    len_bitstream = len(in_stream)
    out_stream += to_gzip_numerical(len_bitstream, 2)
    out_stream += to_gzip_numerical(~len_bitstream, 2)

    out_stream += in_stream

    return out_stream
    

if __name__== "__main__":
    print("starting main ..")

    with open(IN_FILE_PATH, "rb") as file:
        in_stream = file.read()
    
    header = build_header()
    payload = block_type_0(in_stream=in_stream, is_last=1)
    footer = build_footer(in_stream)

    out_stream = header + payload + footer

    print("bitstream:")
    for byte in out_stream:
        print("0b" + ("00000000" + str(bin(byte))[2:])[-8:],hex(byte),  byte)

    with open(OUT_FILE_PATH, "wb") as file:
        file.write(out_stream)