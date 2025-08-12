# Python test sample with various constructs
import asyncio
from typing import List, Dict
import json

class DataProcessor:
    """A sample data processing class."""
    
    def __init__(self, name: str):
        self.name = name
        self.data = []
    
    @staticmethod
    def create_default():
        return DataProcessor("default")
    
    async def process_data(self, items: List[Dict]) -> Dict:
        """Process data asynchronously."""
        results = {}
        for item in items:
            processed = await self.process_item(item)
            results[item['id']] = processed
        return results
    
    async def process_item(self, item: Dict) -> str:
        """Process a single item."""
        await asyncio.sleep(0.01)  # Simulate async work
        return f"processed_{item.get('name', 'unknown')}"
    
    def get_stats(self) -> Dict[str, int]:
        return {
            'total_items': len(self.data),
            'name_length': len(self.name)
        }

# Top-level functions
def main():
    processor = DataProcessor("test")
    data = [{'id': 1, 'name': 'item1'}, {'id': 2, 'name': 'item2'}]
    
    async def run():
        result = await processor.process_data(data)
        print(json.dumps(result, indent=2))
    
    asyncio.run(run())

if __name__ == "__main__":
    main()