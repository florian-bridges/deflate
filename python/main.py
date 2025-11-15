
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

GZIP_HEADER_MTIME = 0x00000000
GZIP_HEADER_XFL = 0x00
GZIP_HEADER_OS = 0x03


def build_header(gzip_compression_method=GZIP_HEADER_CM_DEFLATE):
    
    gzip_header_flag = 0x00
    gzip_header_flag |= GZIP_HEADER_FLG_FTEXT * 0x01
    gzip_header_flag |= GZIP_HEADER_FLG_FHCRC * 0x02
    gzip_header_flag |= GZIP_HEADER_FLG_FEXTRA * 0x04
    gzip_header_flag |= GZIP_HEADER_FLG_FNAME * 0x08
    gzip_header_flag |= GZIP_HEADER_FLG_FCOMMENT * 0x10

    header = bytes([
        GZIP_HEADER_ID1,
        GZIP_HEADER_ID2,
        gzip_compression_method,
        gzip_header_flag,
        GZIP_HEADER_MTIME,
        GZIP_HEADER_MTIME,
        GZIP_HEADER_MTIME,
        GZIP_HEADER_MTIME,
        GZIP_HEADER_XFL,
        GZIP_HEADER_OS
    ])

    return header

def build_footer(in_stream):

    out_stream = bytearray()

    crc = zlib.crc32(in_stream)

    out_stream.append(
        crc & 0xFF
    )

    out_stream.append(
        crc >> 8 & 0xFF
    )

    out_stream.append(
        (crc >> 16) & 0xFF
    )

    out_stream.append(
        (crc >> 24) & 0xFF
    )



    len_bitstream = len(in_stream)

    if len_bitstream > 0xFFFFFFFF:
        raise ValueError(f"length of bitstream has to be less than {0xFFFF} bit is {len_bitstream}")
    
    out_stream.append(
        len_bitstream & 0xFF
    )

    out_stream.append(
        (len_bitstream >> 8) & 0xFF
    )

    out_stream.append(
        (len_bitstream >> 16) & 0xFF
    )

    out_stream.append(
        (len_bitstream >> 24) & 0xFF
    )

    return out_stream



def block_type_0(in_stream, is_last=1):

    block_type = 0
    out_stream = bytearray()

    block_header = 0x00
    block_header |= is_last << 0
    block_header |= block_type  << 1

    print("block_header", block_header)

    out_stream.append(block_header)

    len_bitstream = len(in_stream)

    if len_bitstream > 0xFFFF:
        raise ValueError(f"length of bitstream has to be less than {0xFFFF} bit is {len_bitstream}")
    
    out_stream.append(
        len_bitstream & 0x00FF
    )

    out_stream.append(
        (len_bitstream >> 8) & 0xFF
    )

    out_stream.append(
        (~len_bitstream) & 0xFF
    )

    out_stream.append(
        (~len_bitstream >> 8) & 0xFF
    )

    for byte in in_stream:
        out_stream.append(byte)


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
        print(hex(byte), bin(byte), byte)

    with open(OUT_FILE_PATH, "wb") as file:
        file.write(out_stream)