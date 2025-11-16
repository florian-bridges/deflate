
import zlib

from python.BitStream import BitStream


IN_FILE_PATH = "data/hello_world.txt"
OUT_FILE_PATH = "test_out/hello_world.txt.gz"

#IN_FILE_PATH = "data/lorem_ipsum.txt"
#OUT_FILE_PATH = "test_out/lorem_ipsum.txt.gz"


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

# Compact representation of the length code value (257-285), length range and number
# of extra bits to use in LZ77 compression (See Section 3.2.5 of RFC 1951)
LENGTH_CODE_RANGES = [
    [257,0,3,3],     [258,0,4,4],     [259,0,5,5],     [260,0,6,6],     [261,0,7,7],
    [262,0,8,8],     [263,0,9,9],     [264,0,10,10],   [265,1,11,12],   [266,1,13,14],
    [267,1,15,16],   [268,1,17,18],   [269,2,19,22],   [270,2,23,26],   [271,2,27,30],
    [272,2,31,34],   [273,3,35,42],   [274,3,43,50],   [275,3,51,58],   [276,3,59,66],
    [277,4,67,82],   [278,4,83,98],   [279,4,99,114],  [280,4,115,130], [281,5,131,162], 
    [282,5,163,194], [283,5,195,226], [284,5,227,257], [285,0,258,258]
] 
LENGTH_CODES = {}
for code, num_bits, lower_bound, upper_bound in LENGTH_CODE_RANGES:
    for i in range(upper_bound - lower_bound + 1):
        LENGTH_CODES[lower_bound + i] = (code, num_bits, i)

MAX_REF_LENGTH = 258

# Compact representation of the distance code value (0-31), distance range and number
# of extra bits to use in LZ77 compression (See Section 3.2.5 of RFC 1951)
DISTANCE_CODE_RANGES = [
    [0,0,1,1],         [1,0,2,2],          [2,0,3,3],           [3,0,4,4],           [4,1,5,6],
    [5,1,7,8],         [6,2,9,12],         [7,2,13,16],         [8,3,17,24],         [9,3,25,32],
    [10,4,33,48],      [11,4,49,64],       [12,5,65,96],        [13,5,97,128],       [14,6,129,192],
    [15,6,193,256],    [16,7,257,384],     [17,7,385,512],      [18,8,513,768],      [19,8,769,1024],
    [20,9,1025,1536],  [21,9,1537,2048],   [22,10,2049,3072],   [23,10,3073,4096],   [24,11,4097,6144],
    [25,11,6145,8192], [26,12,8193,12288], [27,12,12289,16384], [28,13,16385,24576], [29,13,24577,32768],
]
DISTANCE_CODES = {}
for code, num_bits, lower_bound, upper_bound in DISTANCE_CODE_RANGES:
    for i in range(upper_bound - lower_bound + 1):
        DISTANCE_CODES[lower_bound + i] = (code, num_bits, i)

MAX_REF_DISTANCE = 32768


def build_header():
    
    out_stream = BitStream()

    out_stream.append_numerical(GZIP_HEADER_ID1)
    out_stream.append_numerical(GZIP_HEADER_ID2)
    out_stream.append_numerical(GZIP_HEADER_CM_DEFLATE)
    out_stream.append(GZIP_HEADER_FLG_FTEXT)
    out_stream.append(GZIP_HEADER_FLG_FHCRC)
    out_stream.append(GZIP_HEADER_FLG_FEXTRA)
    out_stream.append(GZIP_HEADER_FLG_FNAME)
    out_stream.append(GZIP_HEADER_FLG_FCOMMENT)
    out_stream.append(0, 3)
    out_stream.append_numerical(GZIP_HEADER_MTIME, 4)
    out_stream.append_numerical(GZIP_HEADER_XFL)
    out_stream.append_numerical(GZIP_HEADER_OS)

    return out_stream.get()

def build_footer(in_stream):

    out_stream = BitStream()

    out_stream.append_numerical(zlib.crc32(in_stream), 4)
    out_stream.append_numerical(len(in_stream), 4)

    return out_stream.get()



def block_type_0(in_stream, is_last=1):

    block_type = 0
    out_stream = BitStream()

    out_stream.append(is_last)
    out_stream.append(block_type, 2)
    out_stream.append(0, 5)

    len_bitstream = len(in_stream)
    out_stream.append_numerical(len_bitstream, 2)
    out_stream.append_numerical(~len_bitstream, 2)

    return out_stream.get() + in_stream

def get_huffman_codes(length_list):

    bl_count = {}
    tree_lengths = {}

    max_num_bits = 0

    for from_label, to_label, num_bits in length_list:

        for label in range(from_label, to_label + 1):
            tree_lengths[label] = num_bits

        if num_bits not in bl_count.keys():
            bl_count[num_bits] = to_label - from_label + 1
        else:
            bl_count[num_bits] += to_label - from_label + 1

        max_num_bits = max(max_num_bits, num_bits)

    code = 0
    next_code = {}
    for bits in range(1, max_num_bits + 1):
        code += bl_count[bits-1] if bits-1 in bl_count.keys() else 0
        code <<= 1
        next_code[bits] = code

    tree_codes = {}
    for n in range(len(tree_lengths)):
        tree_len = tree_lengths[n]
        if tree_len > 0:
            tree_codes[n] = next_code[tree_len]
            next_code[tree_len] += 1
        
    return tree_codes, tree_lengths


def get_default_codes():
    # from_label, to_label, num_bits
    length_list = [
        (0, 143, 8),
        (144, 255, 9),
        (256, 279, 7),
        (280, 287, 8),
    ]

    return get_huffman_codes(length_list)

def get_distance_codes():
    # from_label, to_label, num_bits
    length_list = [
        (0, 32768, 5),
    ]

    return get_huffman_codes(length_list)


def find_reference(byte_stream, byte_idx):
    ref_dist = 3
    ref_length = 0
    while ref_dist >= 0:

        if ref_dist > byte_idx or ref_dist >= MAX_REF_DISTANCE:
            break

        ref_length = 0

        while byte_stream[byte_idx + ref_length] == byte_stream[byte_idx - ref_dist + ref_length]:

            if ref_length >= len(byte_stream) or ref_length > MAX_REF_LENGTH:
                break

            ref_length += 1

        if ref_length >= 3:
            break

        ref_dist += 1
    return ref_length, ref_dist

    

def block_type_1(in_stream, is_last=1):

    tree_codes, tree_lengths = get_default_codes()
    distance_codes, distance_lengths = get_distance_codes()
    
    block_type = 1
    out_stream = BitStream()

    out_stream.append(is_last)
    out_stream.append(block_type, 2)

    # add block termination token
    byte_stream = [int(byte) for byte in in_stream] + [256]

    byte_idx = 0
    while byte_idx < len(byte_stream):
        byte = byte_stream[byte_idx]

        ref_length, ref_dist = find_reference(byte_stream, byte_idx)        
            
        if ref_length >= 3:

            len_label, len_num_bits, len_offset = LENGTH_CODES[ref_length]
            dist_label, dist_num_bits, dist_offset = DISTANCE_CODES[ref_dist]

            total_ref_size = tree_lengths[len_label] + len_num_bits + distance_lengths[dist_label] + dist_num_bits
            if 8 * ref_length > total_ref_size:

                out_stream.append_reverse(tree_codes[len_label], tree_lengths[len_label])
                out_stream.append(len_offset, len_num_bits)

                out_stream.append_reverse(distance_codes[dist_label], distance_lengths[dist_label])
                out_stream.append(dist_offset, dist_num_bits)

                byte_idx += ref_length
                continue
            
 
        out_stream.append_reverse(tree_codes[byte], tree_lengths[byte])
        byte_idx += 1

    return out_stream.get()


if __name__== "__main__":
    print("starting main ..")

    with open(IN_FILE_PATH, "rb") as file:
        in_stream = file.read()
    
    header = build_header()
    payload = block_type_1(in_stream=in_stream, is_last=1)
    footer = build_footer(in_stream)

    out_stream = header + payload + footer

    print("bitstream:")
    for byte in out_stream:
        print("0b" + ("00000000" + str(bin(byte))[2:])[-8:],hex(byte),  byte)

    with open(OUT_FILE_PATH, "wb") as file:
        file.write(out_stream)