package main

import (
    "fmt"
    "log"
    "sync"
    "time"
)

// DataProcessor struct
type DataProcessor struct {
    name string
    data []int
    mu   sync.Mutex
}

// NewDataProcessor creates a new DataProcessor
func NewDataProcessor(name string) *DataProcessor {
    return &DataProcessor{
        name: name,
        data: make([]int, 0),
    }
}

// ProcessData processes a slice of items
func (dp *DataProcessor) ProcessData(items []int) {
    dp.mu.Lock()
    defer dp.mu.Unlock()
    
    for _, item := range items {
        dp.processItem(item)
    }
}

// processItem processes a single item (private method)
func (dp *DataProcessor) processItem(item int) {
    dp.data = append(dp.data, item*2)
}

// GetData returns a copy of the processed data
func (dp *DataProcessor) GetData() []int {
    dp.mu.Lock()
    defer dp.mu.Unlock()
    
    result := make([]int, len(dp.data))
    copy(result, dp.data)
    return result
}

// GetStats returns statistics about the processor
func (dp *DataProcessor) GetStats() map[string]interface{} {
    dp.mu.Lock()
    defer dp.mu.Unlock()
    
    return map[string]interface{}{
        "name":        dp.name,
        "total_items": len(dp.data),
        "last_update": time.Now(),
    }
}

// Processor interface
type Processor interface {
    ProcessData([]int)
    GetData() []int
}

// Top-level functions
func main() {
    processor := NewDataProcessor("test")
    data := []int{1, 2, 3, 4, 5}
    
    processor.ProcessData(data)
    
    result := processor.GetData()
    fmt.Printf("Processed data: %v\n", result)
    
    stats := processor.GetStats()
    for key, value := range stats {
        fmt.Printf("%s: %v\n", key, value)
    }
}