class TestClass:
    def __init__(self):
        pass
    
    def process(self, data):
        return data

def main():
    test = TestClass()
    return test.process("hello")