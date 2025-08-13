// Test file for fixed GitHub Actions workflow
function processData(input) {
    if (!input) return null;
    
    return {
        processed: true,
        data: input.toUpperCase(),
        timestamp: new Date().toISOString()
    };
}

class DataProcessor {
    constructor(options = {}) {
        this.options = {
            debug: false,
            maxRetries: 3,
            ...options
        };
        this.processed = 0;
    }
    
    process(items) {
        if (!Array.isArray(items)) {
            throw new Error('Items must be an array');
        }
        
        const results = items.map(item => {
            this.processed++;
            if (this.options.debug) {
                console.log(`Processing item ${this.processed}:`, item);
            }
            return processData(item);
        });
        
        return results.filter(result => result !== null);
    }
    
    getStats() {
        return {
            processed: this.processed,
            options: this.options
        };
    }
}

// Test the processor
const processor = new DataProcessor({ debug: true });
const testData = ['hello', 'world', 'github', 'actions'];
const results = processor.process(testData);
console.log('Results:', results);
console.log('Stats:', processor.getStats());