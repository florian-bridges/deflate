
import zlib
import heapq

from python.BitStream import BitStream


IN_FILE_PATH = "data/hello_world.txt"
OUT_FILE_PATH = "test_out/hello_world.txt.gz"

IN_FILE_PATH = "data/lorem_ipsum.txt"
OUT_FILE_PATH = "test_out/lorem_ipsum.txt.gz"


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

# Compact representation of the len code value (257-285), len range and number
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

MAX_REF_LEN = 258

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

CL_INDEX = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15]


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

def get_code_lens(len_list):

    code_lens = {}

    for from_label, to_label, num_bits in len_list:
        for label in range(from_label, to_label + 1):
            code_lens[label] = num_bits

    return code_lens

def get_prefix_codes(tree_lens):

    bl_count = {}
    
    max_num_bits = 0

    for _, num_bits in tree_lens.items():
        if num_bits not in bl_count.keys():
            bl_count[num_bits] = 1
        else:
            bl_count[num_bits] += 1

        max_num_bits = max(max_num_bits, num_bits)

    code = 0
    next_code = {}
    for bits in range(1, max_num_bits + 1):
        code += bl_count[bits-1] if bits-1 in bl_count.keys() else 0
        code <<= 1
        next_code[bits] = code

    tree_codes = {}
    for n in range(len(tree_lens)):
        tree_len = tree_lens[n]
        if tree_len > 0:
            tree_codes[n] = next_code[tree_len]
            next_code[tree_len] += 1
        
    return tree_codes


def get_default_ll_codes():
    # from_label, to_label, num_bits
    len_list = [
        (0, 143, 8),
        (144, 255, 9),
        (256, 279, 7),
        (280, 287, 8),
    ]
    tree_lens = get_code_lens(len_list)
    return get_prefix_codes(tree_lens), tree_lens

def get_default_distance_codes():
    # from_label, to_label, num_bits
    len_list = [
        (0, 32768, 5),
    ]
    tree_lens = get_code_lens(len_list)
    return get_prefix_codes(tree_lens), tree_lens


def find_reference(byte_stream, byte_idx):
    ref_dist = 3
    ref_len = 0
    while ref_dist >= 0:

        if ref_dist > byte_idx or ref_dist >= MAX_REF_DISTANCE:
            break

        ref_len = 0

        while byte_stream[byte_idx + ref_len] == byte_stream[byte_idx - ref_dist + ref_len]:

            if ref_len >= len(byte_stream) or ref_len > MAX_REF_LEN:
                break

            ref_len += 1

        if ref_len >= 3:
            break

        ref_dist += 1
    return ref_len, ref_dist

def encode_byte_stream(byte_stream, out_stream, ll_codes, ll_lens, dist_codes, dist_lens):
    byte_idx = 0
    while byte_idx < len(byte_stream):
        byte = byte_stream[byte_idx]

        ref_len, ref_dist = find_reference(byte_stream, byte_idx)        
            
        if ref_len >= 3:

            len_label, len_num_bits, len_offset = LENGTH_CODES[ref_len]
            dist_label, dist_num_bits, dist_offset = DISTANCE_CODES[ref_dist]

            total_ref_size = ll_lens[len_label] + len_num_bits + dist_lens[dist_label] + dist_num_bits
            if 8 * ref_len > total_ref_size:

                out_stream.append_reverse(ll_codes[len_label], ll_lens[len_label])
                out_stream.append(len_offset, len_num_bits)

                out_stream.append_reverse(dist_codes[dist_label], dist_lens[dist_label])
                out_stream.append(dist_offset, dist_num_bits)

                byte_idx += ref_len
                continue
            
 
        out_stream.append_reverse(ll_codes[byte], ll_lens[byte])
        byte_idx += 1

    return out_stream


def block_type_1(in_stream, is_last=1):

    ll_codes, ll_lens = get_default_ll_codes()
    dist_codes, dist_lens = get_default_distance_codes()
    
    block_type = 1
    out_stream = BitStream()

    out_stream.append(is_last)
    out_stream.append(block_type, 2)

    # add block termination token
    byte_stream = [int(byte) for byte in in_stream] + [256]

    out_stream = encode_byte_stream(byte_stream, out_stream, ll_codes, ll_lens, dist_codes, dist_lens)

    return out_stream.get()

# todo limit len
def huffman_canonical(symbol_freqs):
  
    heap = [(freq, [symbol]) for symbol, freq in symbol_freqs.items() if freq > 0]
    heapq.heapify(heap)

    code_lens = {symbol:0 for symbol in symbol_freqs.keys()}

    if len(heap) == 1:
        _, symbols = heap[0]
        code_lens[symbols[0]] = 1
        return code_lens
    
    while len(heap) > 1:
        freq_1, symbols_1 = heapq.heappop(heap)
        freq_2, symbols_2 = heapq.heappop(heap)

        for symbol in symbols_1 + symbols_2:
            code_lens[symbol] += 1

        heapq.heappush(heap, (freq_1 + freq_2, symbols_1 + symbols_2))

    return code_lens

def get_symbol_freqs(codes, max_code):
    freqs = {code:0 for code in range(max_code + 1)}
    for code, _, _ in codes:
        freqs[code] += 1
    
    return freqs
        
def get_ll_and_distance_codes(byte_stream):

    ll_freqs = {symbol:0 for symbol in range(288)}
    dist_freqs = {symbol:0 for symbol in range(32)}

    byte_idx = 0
    while byte_idx < len(byte_stream):
        byte = byte_stream[byte_idx]

        ref_len, ref_dist = find_reference(byte_stream, byte_idx)        
            
        if ref_len >= 3:

            len_label, _, _ = LENGTH_CODES[ref_len]
            dist_label, _, _ = DISTANCE_CODES[ref_dist]

            ll_freqs[len_label] += 1
            dist_freqs[dist_label] += 1
            byte_idx += ref_len
            continue
            
 
        ll_freqs[byte] += 1
        byte_idx += 1

    ll_lens = huffman_canonical(ll_freqs)
    dist_lens = huffman_canonical(dist_freqs)

    ll_codes = get_prefix_codes(ll_lens)
    dist_codes = get_prefix_codes(dist_lens)

    return ll_codes, ll_lens, dist_codes, dist_lens

def remove_trailing_zeros(code_lens):

    short_code_lens = code_lens.copy()
    for key in range(len(short_code_lens) -1 , 0, -1):
        if short_code_lens[key] != 0:
            break

        short_code_lens.pop(key, None)

    return short_code_lens

def get_code_encodings(code_labels):

    code_encodings = []
    byte_idx = 0
    while byte_idx < len(code_labels):

        current_byte = code_labels[byte_idx]

        repeat_idx = 0
        while byte_idx + repeat_idx + 1 < len(code_labels):
            if code_labels[byte_idx + repeat_idx + 1] == current_byte:
                repeat_idx += 1
                continue
            break
        
        byte_idx += 1 + repeat_idx            

        if current_byte == 0 and repeat_idx >= 3:
            while repeat_idx > 0:
                if repeat_idx >= 11:

                    sequence_len = min(repeat_idx + 1, 138)
                    code_encodings.append((18, sequence_len -11, 7))
                    repeat_idx -= sequence_len
                    continue

                sequence_len = min(repeat_idx  + 1, 10)
                code_encodings.append((17, sequence_len -3, 3))
                repeat_idx -= sequence_len
                

        else:
            code_encodings.append((current_byte, 0, 0))
            while repeat_idx > 0:

                if repeat_idx >= 3:
                    sequence_len = min(repeat_idx, 6)
                    code_encodings.append((16, sequence_len -3 , 2))
                    repeat_idx -= sequence_len
                    continue

                code_encodings.append((current_byte, 0, 0))
                repeat_idx -= 1
    
    return code_encodings

def block_type_2(in_stream, is_last=1):
    
    block_type = 2
    out_stream = BitStream()

    out_stream.append(is_last)
    out_stream.append(block_type, 2)

    # add block termination token
    byte_stream = [int(byte) for byte in in_stream] + [256]
    
    ll_codes, ll_lens, dist_codes, dist_lens = get_ll_and_distance_codes(byte_stream)

    ll_lens_short = remove_trailing_zeros(ll_lens)
    dist_lens_short = remove_trailing_zeros(dist_lens)

    code_encodings = get_code_encodings(list(ll_lens_short.values()) + list(dist_lens_short.values()) )

    cl_symbol_freqs = get_symbol_freqs(code_encodings, 18)
    cl_code_lens = huffman_canonical(cl_symbol_freqs)
    cl_codes =  get_prefix_codes(cl_code_lens)


    hlit = len(ll_lens_short) - 257
    hdist = len(dist_lens_short) -1
    hclen = 19 - 4

    out_stream.append(hlit, 5)
    out_stream.append(hdist, 5)
    out_stream.append(hclen, 4)
    
    
    for cl_idx in CL_INDEX:
        if cl_idx not in cl_codes.keys():
            out_stream.append(0, 3)
            continue
        out_stream.append(cl_code_lens[cl_idx], 3)
        
    for label, repeat, num_bits in code_encodings:
        out_stream.append_reverse(cl_codes[label], cl_code_lens[label])
        out_stream.append(repeat, num_bits)
        
    out_stream = encode_byte_stream(byte_stream, out_stream, ll_codes, ll_lens, dist_codes, dist_lens)

    return out_stream.get()


if __name__== "__main__":
    print("starting main ..")

    with open(IN_FILE_PATH, "rb") as file:
        in_stream = file.read()
    
    header = build_header()
    payload = block_type_2(in_stream=in_stream, is_last=1)
    footer = build_footer(in_stream)

    out_stream = header + payload + footer

    print("bitstream:")
    for byte in out_stream:
        print("0b" + ("00000000" + str(bin(byte))[2:])[-8:],hex(byte),  byte)

    with open(OUT_FILE_PATH, "wb") as file:
        file.write(out_stream)
    