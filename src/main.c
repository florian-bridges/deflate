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

void write_footer(BitStream *stream, unsigned char * src, size_t file_size){
    u_int32_t crc = crc32(0L, Z_NULL, 0);
    crc = crc32(crc, src, file_size);
    append_numerical(stream, crc, 4);
    append_numerical(stream, file_size, 4);
    append_numerical(stream, ~file_size, 4);
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
    block_type_0(&stream, src, statbuf.st_size, 0x01);
    write_footer(&stream, src, statbuf.st_size);
    cut_stream(&stream);

    return 0;

}