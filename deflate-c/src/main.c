#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>  
#include <string.h>  
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>
#include <zlib.h>

#include "bitstream.h"

#ifndef DEFLATE_CONSTANTS_H
#define DEFLATE_CONSTANTS_H

static const size_t REF_LENGTH_RANGES_TAB_LEN = 29;
static const size_t REF_DISTANCE_RANGES_TAB_LEN = 30;
enum {
    MAX_REF_LEN = 258
};
enum {
    MAX_REF_DISTANCE = 32768
};

#endif

typedef struct {
    uint16_t code;
    uint8_t  extra_bits;
    uint16_t base_length;
    uint16_t max_length;
} LengthCodeRange;

static const LengthCodeRange REF_LENGTH_RANGES[] = {
    {257, 0,   3,   3}, {258, 0,   4,   4}, {259, 0,   5,   5}, {260, 0,   6,   6}, {261, 0,   7,   7},
    {262, 0,   8,   8}, {263, 0,   9,   9}, {264, 0,  10,  10}, {265, 1,  11,  12}, {266, 1,  13,  14},
    {267, 1,  15,  16}, {268, 1,  17,  18}, {269, 2,  19,  22}, {270, 2,  23,  26}, {271, 2,  27,  30},
    {272, 2,  31,  34}, {273, 3,  35,  42}, {274, 3,  43,  50}, {275, 3,  51,  58}, {276, 3,  59,  66},
    {277, 4,  67,  82}, {278, 4,  83,  98}, {279, 4,  99, 114}, {280, 4, 115, 130}, {281, 5, 131, 162},
    {282, 5, 163, 194}, {283, 5, 195, 226}, {284, 5, 227, 257}, {285, 0, 258, 258}
};

static const LengthCodeRange REF_DISTANCE_RANGES[] = {
    {0,0,1,1},         {1,0,2,2},          {2,0,3,3},           {3,0,4,4},           {4,1,5,6},
    {5,1,7,8},         {6,2,9,12},         {7,2,13,16},         {8,3,17,24},         {9,3,25,32},
    {10,4,33,48},      {11,4,49,64},       {12,5,65,96},        {13,5,97,128},       {14,6,129,192},
    {15,6,193,256},    {16,7,257,384},     {17,7,385,512},      {18,8,513,768},      {19,8,769,1024},
    {20,9,1025,1536},  {21,9,1537,2048},   {22,10,2049,3072},   {23,10,3073,4096},   {24,11,4097,6144},
    {25,11,6145,8192}, {26,12,8193,12288}, {27,12,12289,16384}, {28,13,16385,24576}, {29,13,24577,32768},
};

typedef struct {
    uint16_t codes[288];
    uint16_t lengths[288];
} llCodes;

typedef struct { 
    uint16_t codes[32769];
    uint16_t lengths[32769];
} distCodes;

void err_quit(const char *msg) {
    fprintf(stderr, "%s", msg);
    exit(1);
}

void write_header(BitStream *stream){
    append_numerical(stream, 0x1f, 1);
    append_numerical(stream, 0x8b, 1);
    append_numerical(stream, 0x08, 1);
    append(stream, 0, 8);
    append_numerical(stream, 0, 4);
    append_numerical(stream, 0, 1);
    append_numerical(stream, 0x03, 1);
}

void get_prefix_codes(size_t n, u_int16_t *prefix_codes, uint16_t *code_lens){

    u_int16_t max_num_bits = 0;

    for(size_t i = 0; i < n; i ++){
        if(code_lens[i] > max_num_bits){
            max_num_bits = code_lens[i];
        }
    }

    u_int16_t bl_count[max_num_bits + 1];
    memset(bl_count, 0, sizeof(bl_count));
    for(size_t i = 0; i < n; i ++){
        if(code_lens[i] > 0){
            bl_count[code_lens[i]] ++;
        }
    }

    u_int16_t code = 0;
    u_int16_t next_code[max_num_bits + 1];
    memset(next_code, 0, sizeof(next_code));
    for(int i = 1; i <= max_num_bits; i++){
        code += bl_count[i-1];
        code <<= 1;
        next_code[i] = code;
    }

    for(size_t i = 0; i < n; i ++){
        if(code_lens[i] > 0){
            prefix_codes[i] = next_code[code_lens[i]];
            next_code[code_lens[i]] ++;
        }
    }

}

typedef struct {
    uint16_t start;
    uint16_t end;
    uint8_t bit_length;
} LengthRange;


llCodes get_default_ll_codes(){

    uint16_t len_ll_codes = 288;

    LengthRange len_list[] = {
        {0,   143, 8},
        {144, 255, 9},
        {256, 279, 7},
        {280, 287, 8},
    };
    int len_list_size = 4; 

    llCodes ll_codes;
    
    for(int i = 0; i < len_list_size; i++){
        for (u_int16_t s = len_list[i].start; s <= len_list[i].end; s ++){
            ll_codes.lengths[s] = len_list[i].bit_length; 
        }
    }

    get_prefix_codes(len_ll_codes, &ll_codes.codes[0], &ll_codes.lengths[0]);

    return ll_codes;
}

distCodes get_default_dist_codes(){
    uint16_t len_codes = 32769;

    LengthRange len_list[] = {
        {0,   32768, 5},
    };
    int len_list_size = 1; 

    distCodes dist_codes;
    
    for(int i = 0; i < len_list_size; i++){
        for (u_int16_t s = len_list[i].start; s <= len_list[i].end; s ++){
            dist_codes.lengths[s] = len_list[i].bit_length; 
        }
    }

    get_prefix_codes(len_codes, &dist_codes.codes[0], &dist_codes.lengths[0]);

    return dist_codes;

}

void block_type_0(BitStream *stream, unsigned char* src, size_t file_size, u_int8_t is_last){
    
    u_int8_t block_type = 0;
    
    append(stream, is_last, 1);
    append(stream, block_type, 2);
    append(stream, 0, 5);
    append_numerical(stream, file_size, 2);
    append_numerical(stream, ~file_size, 2);

    for (size_t i = 0; i < file_size; i++) {
        char c = src[i];
        append(stream, c, 8);   
    }

}

typedef struct {
    uint16_t label;
    uint8_t  extra_bits;
    uint16_t offset;
} refLabels;

void set_ref_length_labels(refLabels* length_labels){

    for(size_t i=0; i < REF_LENGTH_RANGES_TAB_LEN; i ++){
        for(size_t j=REF_LENGTH_RANGES[i].base_length; j <= REF_LENGTH_RANGES[i].max_length; j++){
            length_labels[j].label = REF_LENGTH_RANGES[i].code;
            length_labels[j].extra_bits = REF_LENGTH_RANGES[i].extra_bits;
            length_labels[j].offset = j - REF_LENGTH_RANGES[i].base_length;
        }
    }
}

void set_ref_dist_labels(refLabels* dist_labels){
    
    for(size_t i=0; i < REF_DISTANCE_RANGES_TAB_LEN; i ++){
        for(size_t j=REF_DISTANCE_RANGES[i].base_length; j <= REF_DISTANCE_RANGES[i].max_length; j++){
            dist_labels[j].label = REF_DISTANCE_RANGES[i].code;
            dist_labels[j].extra_bits = REF_DISTANCE_RANGES[i].extra_bits;
            dist_labels[j].offset = j - REF_DISTANCE_RANGES[i].base_length;
        }
    }
}

typedef struct {
    u_int32_t length;
    u_int32_t dist;
} Ref;

Ref find_reference(unsigned char* src, size_t input_size,size_t byte_idx){
    size_t ref_dist = 3;
    size_t ref_len = 0;

    while (1)
    {
        if (ref_dist > byte_idx || ref_dist >= MAX_REF_DISTANCE){
            break;
        }

        ref_len = 0;

        while (src[byte_idx + ref_len] == src[byte_idx - ref_dist + ref_len]){
            
            ref_len ++;

            if (ref_len + byte_idx >= input_size || ref_len > MAX_REF_LEN){
                break;
            }
        }
        if (ref_len >= 3){
            break;
        }

        ref_dist ++;
    }

    Ref ref; 
    ref.dist = ref_dist;
    ref.length = ref_len;

    return ref;
    
}

void encode_byte_stream(BitStream *stream, unsigned char* src, size_t input_size, llCodes ll_codes, distCodes dist_codes){


    refLabels ref_length_labels[MAX_REF_LEN];  
    set_ref_length_labels(&ref_length_labels);

    refLabels ref_dist_labels[MAX_REF_DISTANCE];  
    set_ref_dist_labels(&ref_dist_labels);


    u_int8_t byte = 0; 
    size_t byte_idx = 0;
    while ( byte_idx < input_size)
    {
        byte = src[byte_idx];

        Ref ref = find_reference(src, input_size, byte_idx);

        if(ref.length >= 3){

            size_t len_label = (size_t) ref_length_labels[(size_t)ref.length].label;
            size_t len_offset = (size_t) ref_length_labels[(size_t)ref.length].offset;
            size_t len_num_bits = (size_t) ref_length_labels[(size_t)ref.length].extra_bits;

            size_t dist_label = (size_t) ref_dist_labels[(size_t)ref.dist].label;
            size_t dist_offset = (size_t) ref_dist_labels[(size_t)ref.dist].offset;
            size_t dist_num_bits = (size_t) ref_dist_labels[(size_t)ref.dist].extra_bits;

            append_reverse(stream, ll_codes.codes[len_label], ll_codes.lengths[len_label]);
            append(stream, len_offset, len_num_bits);

            append_reverse(stream, dist_codes.codes[dist_label], dist_codes.lengths[dist_label]);
            append(stream, dist_offset, dist_num_bits);

            byte_idx += ref.length;
            continue;

        }

        append_reverse(stream, ll_codes.codes[(size_t)byte],ll_codes.lengths[(size_t)byte]);

        printf("%d %d %c %d %d\n", byte_idx, byte, (char) byte, ll_codes.codes[(size_t)byte], ll_codes.lengths[(size_t)byte]);
        byte_idx ++; 
        
    }
    
    // encode block termination
    append_reverse(stream, ll_codes.codes[(size_t)256],ll_codes.lengths[(size_t)256]);

}

void block_type_1(BitStream *stream, unsigned char* src, size_t input_size, u_int8_t is_last){
    
    llCodes ll_codes = get_default_ll_codes();
    distCodes dist_codes = get_default_dist_codes();

    u_int8_t block_type = 1;
    
    append(stream, is_last, 1);
    append(stream, block_type, 2);
    
    encode_byte_stream(stream, src, input_size, ll_codes, dist_codes);
    jump_to_next_byte(stream);

}

void write_footer(BitStream *stream, unsigned char * src, size_t file_size){
    u_int32_t crc = crc32(0L, Z_NULL, 0);
    crc = crc32(crc, src, file_size);
    append_numerical(stream, crc, 4);
    append_numerical(stream, file_size, 4);
}

int main(int argc, char *argv[]) {
    int fdin;
    unsigned char *src;
    struct stat statbuf;

    if (argc != 3) err_quit("usage: deflate <fromfile> <tofile>");

    printf("starting deflate ..\n");

    fdin = open(argv[1], O_RDONLY);
    if (fdin < 0) {
        printf("can't open %s for reading", argv[1]);
        return 0;
    }

    if (fstat(fdin, &statbuf) < 0) {
        printf("fstat error");
        return 0;
    }

    src = get_mmap(statbuf.st_size, PROT_READ, fdin);
    BitStream stream = get_write_stream(argv[2]);
        

    write_header(&stream);
    block_type_1(&stream, src, statbuf.st_size, 0x01);
    write_footer(&stream, src, statbuf.st_size);
    cut_stream(&stream);

    return 0;

}