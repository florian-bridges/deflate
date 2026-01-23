
class BitStream:

    def __init__(self):

        self.bit_stream = bytearray()
        self.new_byte = 0x00
        self.bit_idx = 0
        pass

    def reverse_bits(self, bits, num_bits):
        result = 0
        for _ in range(num_bits):
            result = (result << 1) | (bits & 1)
            bits >>= 1
        return result

    def append_bit(self, bit):
        if self.bit_idx == 8:
            self.bit_stream.append(self.new_byte)
            self.new_byte = 0x00
            self.bit_idx = 0

        self.new_byte |= (bit & 0x01) << self.bit_idx
        self.bit_idx += 1

    def append(self, bits, num_bits=1):        
        for _ in range(num_bits):
            self.append_bit(
                bits & 0x01
            )

            bits >>= 1

    def append_reverse(self, bits, num_bits=1):
        bits = self.reverse_bits(bits, num_bits)
        self.append(bits, num_bits)

    def append_numerical(self, value, num_bytes=1):

        if value > 2**(num_bytes * 8):
            raise ValueError(f"length of bitstream has to be less than {2**(num_bytes * 8)} bit is {value}")

        for i in range(num_bytes):
            self.append((value >> (8 * i)) & 0xFF, 8)

    def get(self):
        if self.bit_idx == 0:
            return self.bit_stream
        
        return self.bit_stream + bytearray([self.new_byte])

