#ifndef BITSTREAM_H
#define BITSTREAM_H

#include <stdint.h>

typedef struct {
    int fd;
    unsigned char *start;
    size_t bytepos; 
    uint8_t bitpos;
    size_t max_size;
} BitStream;

unsigned char * get_mmap(size_t file_size, int prot, int fd);
BitStream get_write_stream(char * file_name);
void jump_to_next_byte(BitStream *stream);
void cut_stream(BitStream *stream);
void append_bit(BitStream *stream, uint8_t bit);
void append(BitStream *stream, uint16_t bits, uint16_t num_bits);
void append_reverse(BitStream *stream, uint16_t bits, uint16_t num_bits);
void append_numerical(BitStream *stream, uint32_t bits, uint16_t num_bytes);


#endif

