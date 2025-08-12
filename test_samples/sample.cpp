// C++ test sample with various constructs
#include <iostream>
#include <vector>
#include <memory>
#include <string>

class DataProcessor {
private:
    std::string name;
    std::vector<int> data;

public:
    DataProcessor(const std::string& name) : name(name) {}
    
    virtual ~DataProcessor() = default;
    
    static std::unique_ptr<DataProcessor> create_default() {
        return std::make_unique<DataProcessor>("default");
    }
    
    virtual void process_data(const std::vector<int>& items) {
        for (const auto& item : items) {
            process_item(item);
        }
    }
    
    inline void process_item(int item) {
        data.push_back(item * 2);
    }
    
    const std::vector<int>& get_data() const {
        return data;
    }
};

class AdvancedProcessor : public DataProcessor {
public:
    AdvancedProcessor() : DataProcessor("advanced") {}
    
    void process_data(const std::vector<int>& items) override {
        std::cout << "Advanced processing..." << std::endl;
        DataProcessor::process_data(items);
    }
};

struct SimpleData {
    int id;
    std::string name;
    
    SimpleData(int id, const std::string& name) : id(id), name(name) {}
};

// Top-level functions
int main() {
    auto processor = DataProcessor::create_default();
    std::vector<int> data = {1, 2, 3, 4, 5};
    
    processor->process_data(data);
    
    const auto& result = processor->get_data();
    for (const auto& item : result) {
        std::cout << item << " ";
    }
    std::cout << std::endl;
    
    return 0;
}