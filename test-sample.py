# Test Python file to trigger multi-language detection
def calculate_area(length, width):
    """Calculate the area of a rectangle"""
    return length * width

class Rectangle:
    def __init__(self, length, width):
        self.length = length
        self.width = width
        
    def get_area(self):
        return calculate_area(self.length, self.width)
    
    def get_perimeter(self):
        return 2 * (self.length + self.width)

# Test the Rectangle class
rect = Rectangle(5, 10)
print(f"Area: {rect.get_area()}")
print(f"Perimeter: {rect.get_perimeter()}")