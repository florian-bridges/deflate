#define _GNU_SOURCE

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <unistd.h>
#include <fcntl.h>


#include "bitstream.h"


unsigned char * get_mmap(size_t file_size, int prot, int fd){

    unsigned char * mem_map; 

    mem_map = mmap(0, file_size, prot, MAP_SHARED, fd, 0);
    if (mem_map == MAP_FAILED) {
        printf("mmap error for input");
        return 0;
    }

    return mem_map;

}

BitStream get_write_stream(char * file_name){

    int mode = 0777;
    size_t default_file_size = 1024;
    
    int fdout = open(file_name, O_RDWR | O_CREAT | O_TRUNC, mode);
    if (fdout < 0) {
        printf("can't create %s for writing", file_name);
        exit(1);
    }

    if (ftruncate(fdout, default_file_size) < 0) {
        perror("ftruncate");
        exit(1);
    }
    
    unsigned char* dst = get_mmap(default_file_size, PROT_READ | PROT_WRITE, fdout);

    BitStream stream = { 
        .fd = fdout,
        .start = dst,
        .bytepos = 0,
        .bitpos = 0,
        .max_size = default_file_size
    };
    dst[0] = 0;

    return stream;
}


void grow_stream(BitStream *stream){
    printf("increasing file_size\n");

    size_t new_max_size = stream->max_size + 1024; 
    ftruncate(stream->fd, new_max_size);
    stream->start = mremap(stream->start, stream->max_size, new_max_size, MREMAP_MAYMOVE);
    if (stream->start == MAP_FAILED) {
        perror("mremap");
        exit(1);
    }
    stream->max_size = new_max_size;
}

void cut_stream(BitStream *stream){
    size_t stream_size = stream->bytepos + 1; 
    ftruncate(stream->fd, stream_size);
    stream->start = mremap(stream->start, stream->max_size, stream_size, MREMAP_MAYMOVE);
    if (stream->start == MAP_FAILED) {
        perror("mremap");
        exit(1);
    }
    stream->max_size = stream_size;
}

void append_bit(BitStream *stream, uint8_t bit){

    bit = bit & 0x01;

    *(stream->start+ stream->bytepos) |= bit << stream->bitpos;
    
    stream->bitpos++;
    if (stream->bitpos == 8) {
        stream->bitpos = 0;
        stream->bytepos++;

        if (stream->bytepos == stream->max_size) {  
            grow_stream(stream);
        }
        *(stream->start + stream->bytepos) = 0;
    }   
}


void append(BitStream *stream, uint16_t bits, uint16_t num_bits){
    for(int i = 0; i < num_bits; i++){
        append_bit(stream, (bits >> i) & 0x01);
    }
}

uint16_t reverse_bits(uint16_t bits, uint16_t num_bits){
    
    uint16_t reverse_bits = 0; 
    
    for(int i = 0; i < num_bits; i++){
        reverse_bits = (reverse_bits << 1) | (bits & 0x01);
        bits = bits >> 1;
    }
    return reverse_bits;
}

void append_reverse(BitStream *stream, uint16_t bits, uint16_t num_bits){
    bits = reverse_bits(bits, num_bits);
    append(stream, bits, num_bits);
}


void append_numerical(BitStream *stream, uint32_t bits, uint16_t num_bytes){
    
    for(int i = 0; i < num_bytes; i++){
        append(stream, bits >> (8*i) & 0xFF, 8);
    }
}