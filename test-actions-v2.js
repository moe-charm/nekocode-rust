// Test JavaScript file for GitHub Actions v2
function greetUser(name) {
    return `Hello, ${name}! Welcome to NekoCode.`;
}

class UserManager {
    constructor() {
        this.users = [];
        this.activeCount = 0;
    }
    
    addUser(name) {
        this.users.push({
            name: name,
            joinedAt: new Date(),
            active: true
        });
        this.activeCount++;
        return greetUser(name);
    }
    
    removeUser(name) {
        const index = this.users.findIndex(u => u.name === name);
        if (index >= 0) {
            this.users[index].active = false;
            this.activeCount--;
            return `${name} has been removed.`;
        }
        return `${name} not found.`;
    }
}

// Test the UserManager
const manager = new UserManager();
console.log(manager.addUser('Alice'));
console.log(manager.addUser('Bob'));
console.log(manager.removeUser('Alice'));