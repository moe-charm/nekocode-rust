#include <iostream>
#include <vector>
#include "custom_header.h"

using namespace std;

template<typename T>
class Vector {
private:
    vector<T> data;
    
public:
    Vector() {}
    
    virtual ~Vector() {}
    
    void push(const T& item) {
        data.push_back(item);
        if (data.size() > 10) {
            // Complex logic
            for (auto& elem : data) {
                if (elem > 0) {
                    elem *= 2;
                }
            }
        }
    }
    
    virtual int size() const {
        return data.size();
    }
};

int fibonacci(int n) {
    if (n <= 1) return n;
    return fibonacci(n-1) + fibonacci(n-2);
}

int main() {
    Vector<int> vec;
    vec.push(42);
    cout << "Size: " << vec.size() << endl;
    return 0;
}