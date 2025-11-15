#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>   /* exit */
#include <string.h>   /* memcpy */
#include <sys/mman.h> /* mmap() is defined in this header */
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h> /* lseek, write, close */

void err_quit(const char *msg) {
    fprintf(stderr, "%s", msg);
    exit(1);
}

char * get_mmap(size_t file_size, int prot, int fd){

    char * mem_map; 

    mem_map = mmap(0, file_size, prot, MAP_SHARED, fd, 0);
    if (mem_map == MAP_FAILED) {
        printf("mmap error for input");
        return 0;
    }

    return mem_map;

}

int main(int argc, char *argv[]) {
    int fdin, fdout;
    char *src, *dst;
    struct stat statbuf;
    int mode = 0777;

    if (argc != 3) err_quit("usage: deflate <fromfile> <tofile>");

    /* open the input file */
    fdin = open(argv[1], O_RDONLY);
    if (fdin < 0) {
        printf("can't open %s for reading", argv[1]);
        return 0;
    }

    /* open/create the output file */
    fdout = open(argv[2], O_RDWR | O_CREAT | O_TRUNC, mode);
    if (fdout < 0) {
        printf("can't create %s for writing", argv[2]);
        return 0;
    }

    /* find size of input file */
    if (fstat(fdin, &statbuf) < 0) {
        printf("fstat error");
        return 0;
    }

    /* go to the location corresponding to the last byte */
    if (lseek(fdout, statbuf.st_size - 1, SEEK_SET) == -1) {
        printf("lseek error");
        return 0;
    }

    /* write a dummy byte at the last location */
    if (write(fdout, "", 1) != 1) {
        printf("write error");
        return 0;
    }

    
    /* mmap the input file */
    src = get_mmap(statbuf.st_size, PROT_READ, fdin);

    /* mmap the output file */
    dst = get_mmap(statbuf.st_size, PROT_READ | PROT_WRITE, fdout);

    for (size_t i = 0; i < statbuf.st_size; i++) {
        char c = src[i];  // get the i-th byte
        printf("Byte %zu: %c (0x%02x)\n", i, 
               (c >= 32 && c <= 126) ? c : '.',  // printable or dot
               (unsigned char)c);                 // show hex value

        dst[i] = src[i];
    }


    return 0;

} /* main */