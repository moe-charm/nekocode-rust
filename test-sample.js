// Test JavaScript file to trigger GitHub Actions
function testFunction(x, y) {
    console.log('Testing NekoCode PR analysis');
    return x + y;
}

class TestClass {
    constructor(name) {
        this.name = name;
        this.count = 0;
    }
    
    increment() {
        this.count++;
        return this.count;
    }
    
    reset() {
        this.count = 0;
        console.log('Counter reset for', this.name);
    }
}

const test = new TestClass('NekoCode Test');
console.log(testFunction(10, 20));
console.log(test.increment());