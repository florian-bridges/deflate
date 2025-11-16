
class BitStream:

    def __init__(self):

        self.bit_stream = bytearray()
        self.new_byte = 0x00
        self.bit_idx = 0
        pass

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

    def get(self):
        if self.bit_idx == 0:
            return self.bit_stream
        
        return self.bit_stream + bytearray([self.new_byte])

