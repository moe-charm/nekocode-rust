# Final test for GitHub Actions NekoCode workflow
def calculate_fibonacci(n):
    """Calculate Fibonacci sequence up to n terms"""
    if n <= 0:
        return []
    elif n == 1:
        return [0]
    elif n == 2:
        return [0, 1]
    
    fib = [0, 1]
    for i in range(2, n):
        fib.append(fib[i-1] + fib[i-2])
    return fib

class MathUtils:
    """Utility class for mathematical operations"""
    
    def __init__(self):
        self.history = []
    
    def factorial(self, n):
        """Calculate factorial of n"""
        if n < 0:
            raise ValueError("Factorial is not defined for negative numbers")
        if n == 0 or n == 1:
            return 1
        
        result = 1
        for i in range(2, n + 1):
            result *= i
        
        self.history.append(f"factorial({n}) = {result}")
        return result
    
    def prime_check(self, num):
        """Check if a number is prime"""
        if num < 2:
            return False
        for i in range(2, int(num ** 0.5) + 1):
            if num % i == 0:
                return False
        return True
    
    def get_primes_up_to(self, limit):
        """Get all prime numbers up to limit"""
        primes = []
        for num in range(2, limit + 1):
            if self.prime_check(num):
                primes.append(num)
        return primes

# Test the classes and functions
if __name__ == "__main__":
    print("🧮 Testing Fibonacci sequence:")
    fib_result = calculate_fibonacci(10)
    print(f"First 10 Fibonacci numbers: {fib_result}")
    
    print("\n🔢 Testing MathUtils:")
    math = MathUtils()
    
    print(f"Factorial of 5: {math.factorial(5)}")
    print(f"Is 17 prime? {math.prime_check(17)}")
    print(f"Primes up to 20: {math.get_primes_up_to(20)}")
    
    print(f"\n📝 Calculation history: {math.history}")